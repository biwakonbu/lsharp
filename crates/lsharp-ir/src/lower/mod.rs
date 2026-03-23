//! Typed AST -> IR 変換 (Lowering)

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::types::Type;

use crate::{GcField, GcTypeDef, GcTypeKind, IrType, Module};

mod decl;
mod expr;
mod pattern;
#[cfg(test)]
mod tests;

/// Lowering エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum LowerError {
    #[error("未サポートの式: {msg}")]
    Unsupported { msg: String },

    #[error("未定義の関数: {name}")]
    UndefinedFunction { name: String },
}

/// Lowering コンテキスト
pub struct Lower {
    /// 関数名 -> 関数インデックスのマッピング
    pub(crate) func_indices: HashMap<String, u32>,
    /// import 関数の数（ユーザー関数のインデックスオフセット）
    pub(crate) import_count: u32,
    /// 型推論結果
    pub(crate) type_results: HashMap<String, Type>,
    /// レコード型名 -> GC 型インデックス
    pub(crate) record_type_indices: HashMap<String, u32>,
    /// レコード型名 -> フィールド名リスト（順序保持）
    pub(crate) record_fields: HashMap<String, Vec<String>>,
    /// GC 型定義のリスト
    pub(crate) gc_types: Vec<GcTypeDef>,
    /// トレイト実装メソッドの解決テーブル
    /// (trait_name, type_name, method_name) -> 関数名
    pub(crate) trait_method_impls: HashMap<(String, String, String), String>,
    /// トレイトメソッド名 -> トレイト名の逆引きテーブル（静的ディスパッチ用）
    /// method_name -> Vec<trait_name>
    pub(crate) trait_method_names: HashMap<String, Vec<String>>,
    /// 制約付き型の登録情報 (ランタイム検証用)
    #[allow(dead_code)]
    pub(crate) constrained_type_checks: HashMap<String, Vec<(String, i64, i64)>>,
    /// ADT バリアント名 -> (GC 型インデックス, タグ値)
    pub(crate) adt_variant_indices: HashMap<String, (u32, i32)>,
    /// ADT 型名 -> バリアント情報リスト [(name, gc_idx, tag, field_count)]
    pub(crate) adt_type_info: HashMap<String, Vec<(String, u32, i32, usize)>>,
    /// 文字列定数データ [(label, bytes)]
    pub(crate) string_data: RefCell<Vec<(String, Vec<u8>)>>,
    /// 次の文字列データオフセット
    pub(crate) string_offset: Cell<u32>,
    /// Computation Builder 情報（ビルダー名 -> (bind関数名, return関数名)）
    pub(crate) computation_builders: HashMap<String, (String, String)>,
    /// Lambda Lifting: リフトされた関数のリスト
    pub(crate) lifted_functions: RefCell<Vec<crate::Function>>,
    /// Lambda Lifting: 一意な Lambda 名生成用カウンター
    pub(crate) lambda_counter: Cell<u32>,
    /// Lambda Lifting: リフトされた関数名 -> 関数インデックスのマッピング
    pub(crate) lifted_func_indices: RefCell<HashMap<String, u32>>,
    /// Lambda Lifting: 次に割り当てる関数インデックス
    pub(crate) next_func_idx: Cell<u32>,
}

/// Private 宣言を展開して内部の宣言を返す
pub(crate) fn unwrap_private(decl: &Decl) -> &Decl {
    match decl {
        Decl::Private { inner, .. } => unwrap_private(inner),
        other => other,
    }
}

impl Lower {
    pub fn new() -> Self {
        Self {
            func_indices: HashMap::new(),
            import_count: 0,
            type_results: HashMap::new(),
            record_type_indices: HashMap::new(),
            record_fields: HashMap::new(),
            gc_types: Vec::new(),
            trait_method_impls: HashMap::new(),
            trait_method_names: HashMap::new(),
            constrained_type_checks: HashMap::new(),
            adt_variant_indices: HashMap::new(),
            adt_type_info: HashMap::new(),
            string_data: RefCell::new(Vec::new()),
            string_offset: Cell::new(512), // 文字列データの開始位置（メモリ先頭は数値変換バッファ用）
            computation_builders: HashMap::new(),
            lifted_functions: RefCell::new(Vec::new()),
            lambda_counter: Cell::new(0),
            lifted_func_indices: RefCell::new(HashMap::new()),
            next_func_idx: Cell::new(0),
        }
    }

    /// 一意な Lambda 関数名を生成
    pub(crate) fn fresh_lambda_name(&self) -> String {
        let id = self.lambda_counter.get();
        self.lambda_counter.set(id + 1);
        format!("__lambda_{id}")
    }

    /// プログラム全体を IR に変換
    pub fn lower_program(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
    ) -> Result<Module, LowerError> {
        // 型推論結果を保存
        for (name, scheme) in type_results {
            self.type_results.insert(name.clone(), scheme.ty.clone());
        }

        // レコード型定義を GC 型として登録
        for decl in &program.decls {
            if let Decl::RecordDef {
                name, fields, ..
            } = unwrap_private(decl)
            {
                let gc_idx = self.gc_types.len() as u32;
                self.record_type_indices.insert(name.clone(), gc_idx);

                let gc_fields: Vec<GcField> = fields
                    .iter()
                    .map(|(fname, ftype)| GcField {
                        name: fname.clone(),
                        ty: type_expr_to_ir(ftype),
                        mutable: false,
                    })
                    .collect();

                // フィールド名リストを記録
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                self.record_fields.insert(name.clone(), field_names);

                self.gc_types.push(GcTypeDef {
                    name: name.clone(),
                    kind: GcTypeKind::Struct(gc_fields),
                });
            }
        }

        // ADT 型定義のバリアント情報を登録
        // リニアメモリ版では GC 型は不要 (ヒープに直接書き込む)
        for decl in &program.decls {
            if let Decl::TypeDef {
                name, variants, ..
            } = unwrap_private(decl)
            {
                let mut variant_infos = Vec::new();
                for (tag, variant) in variants.iter().enumerate() {
                    let tag_val = tag as i32;
                    // gc_idx は互換性のため 0 を設定 (リニアメモリ版では未使用)
                    self.adt_variant_indices.insert(variant.name.clone(), (0, tag_val));
                    variant_infos.push((variant.name.clone(), 0, tag_val, variant.fields.len()));
                }
                self.adt_type_info.insert(name.clone(), variant_infos);
            }
        }

        // import/内部ヘルパー関数を登録
        self.func_indices.insert("print".to_string(), 0);
        self.func_indices.insert("__alloc".to_string(), 1);
        self.func_indices.insert("__string_concat".to_string(), 2);
        self.func_indices.insert("__string_eq".to_string(), 3);
        self.func_indices.insert("print-string".to_string(), 4);
        self.func_indices.insert("proc-exit".to_string(), 5);
        self.import_count = 6;

        // ユーザー定義関数のインデックスを事前登録
        let mut func_idx = self.import_count;
        for decl in &program.decls {
            if let Decl::Defn { name, .. } = unwrap_private(decl) {
                self.func_indices.insert(name.clone(), func_idx);
                func_idx += 1;
            }
        }

        // フィールドアクセサ関数のインデックスを登録
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                for (fname, _) in fields {
                    let accessor_name = format!("{name}.{fname}");
                    self.func_indices.insert(accessor_name, func_idx);
                    func_idx += 1;
                }
            }
        }

        // ADT コンストラクタ関数のインデックスを登録
        for decl in &program.decls {
            if let Decl::TypeDef { variants, .. } = unwrap_private(decl) {
                for variant in variants {
                    self.func_indices.insert(variant.name.clone(), func_idx);
                    func_idx += 1;
                }
            }
        }

        // トレイト定義からメソッド名の逆引きテーブルを構築（P5-6: 静的ディスパッチ）
        for decl in &program.decls {
            if let Decl::TraitDef {
                name: trait_name,
                methods,
                ..
            } = unwrap_private(decl)
            {
                for method in methods {
                    self.trait_method_names
                        .entry(method.name.clone())
                        .or_default()
                        .push(trait_name.clone());
                }
            }
        }

        // トレイト実装メソッドのインデックスを登録 (P5-6: 辞書パスイング)
        for decl in &program.decls {
            if let Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } = unwrap_private(decl)
            {
                for method_decl in methods {
                    if let Decl::Defn { name: method_name, .. } = unwrap_private(method_decl) {
                        // マングル名: TraitName_TypeName_methodName
                        let mangled = format!("{trait_name}_{type_name}_{method_name}");
                        self.func_indices.insert(mangled.clone(), func_idx);
                        self.trait_method_impls.insert(
                            (trait_name.clone(), type_name.clone(), method_name.clone()),
                            mangled,
                        );
                        func_idx += 1;
                    }
                }
            }
        }

        // Computation Builder の登録
        for decl in &program.decls {
            if let Decl::ComputationBuilder { name, bind_fn, return_fn, .. } = unwrap_private(decl) {
                self.computation_builders.insert(
                    name.clone(),
                    (bind_fn.clone(), return_fn.clone()),
                );
            }
        }

        // Lambda Lifting 用の次の関数インデックスを設定
        self.next_func_idx.set(func_idx);

        // 各関数を IR に変換
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::Defn {
                name, params, body, ..
            } = unwrap_private(decl)
            {
                let func = self.lower_function(name, params, body)?;
                functions.push(func);
            }
        }

        // フィールドアクセサ関数を生成
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                for (field_idx, (fname, ftype)) in fields.iter().enumerate() {
                    let accessor = self.generate_field_accessor(name, fname, field_idx as u32, ftype);
                    functions.push(accessor);
                }
            }
        }

        // トレイト実装メソッドを IR 関数として生成 (P5-6)
        for decl in &program.decls {
            if let Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } = unwrap_private(decl)
            {
                for method_decl in methods {
                    if let Decl::Defn {
                        name: method_name,
                        params,
                        body,
                        ..
                    } = unwrap_private(method_decl)
                    {
                        let mangled = format!("{trait_name}_{type_name}_{method_name}");
                        let func = self.lower_function(&mangled, params, body)?;
                        functions.push(func);
                    }
                }
            }
        }

        // 制約付き型のランタイム検証関数を生成 (P2-6)
        for decl in &program.decls {
            if let Decl::TypeConstrained {
                name,
                constraints,
                ..
            } = unwrap_private(decl)
            {
                // Name.new: (-> BaseType BaseType) -- 制約チェック付き
                let check_func = self.generate_constraint_check(name, constraints);
                // 関数インデックスを登録
                let check_name = format!("{name}.new");
                if !self.func_indices.contains_key(&check_name) {
                    self.func_indices.insert(check_name.clone(), func_idx);
                    func_idx += 1;
                }
                functions.push(check_func);

                // Name.valid?: (-> BaseType Bool) -- 検証のみ（トラップしない）
                let valid_func = self.generate_constraint_valid(name, constraints);
                let valid_name = format!("{name}.valid?");
                if !self.func_indices.contains_key(&valid_name) {
                    self.func_indices.insert(valid_name.clone(), func_idx);
                    func_idx += 1;
                }
                functions.push(valid_func);
            }
        }

        // ADT コンストラクタ関数を生成 (P1-9)
        for decl in &program.decls {
            if let Decl::TypeDef { variants, .. } = unwrap_private(decl) {
                for variant in variants {
                    if let Some(&(gc_idx, tag_val)) = self.adt_variant_indices.get(&variant.name) {
                        let ctor = self.generate_adt_constructor(
                            &variant.name,
                            gc_idx,
                            tag_val,
                            variant.fields.len(),
                        );
                        functions.push(ctor);
                    }
                }
            }
        }

        // func_idx の未使用警告を抑制
        let _ = func_idx;

        // Lambda Lifting: リフトされた関数を追加
        let lifted = self.lifted_functions.borrow().clone();
        functions.extend(lifted);

        Ok(Module {
            functions,
            gc_types: self.gc_types.clone(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: self.string_data.borrow().clone(),
        })
    }
}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}

/// 関数変換コンテキスト
pub(crate) struct FuncCtx {
    #[allow(dead_code)]
    pub(crate) name: String,
    pub(crate) instructions: Vec<crate::Instruction>,
    pub(crate) locals_map: HashMap<String, u32>,
    pub(crate) param_count: u32,
    pub(crate) next_local: u32,
}

impl FuncCtx {
    pub(crate) fn new(name: String) -> Self {
        Self {
            name,
            instructions: Vec::new(),
            locals_map: HashMap::new(),
            param_count: 0,
            next_local: 0,
        }
    }

    pub(crate) fn emit(&mut self, instr: crate::Instruction) {
        self.instructions.push(instr);
    }

    pub(crate) fn alloc_local(&mut self, name: String) -> u32 {
        if let Some(&idx) = self.locals_map.get(&name) {
            return idx;
        }
        let idx = self.next_local;
        self.locals_map.insert(name, idx);
        self.next_local += 1;
        idx
    }
}

/// 組み込み二項演算子か判定
pub(crate) fn is_builtin_binop(name: &str) -> bool {
    matches!(
        name,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "+."
            | "-."
            | "*."
            | "/."
            | "==" | "="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "and"
            | "or"
    )
}

/// L# 型 -> IR 型
pub fn type_to_ir(ty: &Type) -> IrType {
    match ty {
        Type::Con(name) => match name.as_str() {
            "Int" => IrType::I64,
            "Float" => IrType::F64,
            "Bool" => IrType::I64, // Bool は i64 (0/1)
            "Unit" => IrType::I64, // Unit も i64 (0)
            "String" => IrType::I64, // MVP: 文字列はポインタ (i64)
            _ => IrType::I64,
        },
        Type::Var(_) => IrType::I64, // 未解決の型変数はデフォルト i64
        Type::Fun(_, _) => IrType::I64, // 関数ポインタ
        Type::App(_, _) => IrType::I64, // ADT ポインタ
        Type::Record(_, _) => IrType::I64, // MVP: レコードは i64
    }
}

/// 型から具体型名を抽出（静的ディスパッチ用）
pub(crate) fn type_to_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Con(name) => Some(name.clone()),
        Type::Record(name, _) => Some(name.clone()),
        Type::App(name, _) => Some(name.clone()),
        _ => None,
    }
}

/// TypeExpr から型名を抽出（静的ディスパッチ用）
pub(crate) fn type_expr_to_name(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(_, name) => Some(name.clone()),
        _ => None,
    }
}

// === ヒープオブジェクトタグ定数 ===

/// ヒープオブジェクトタグ: 文字列
pub const HEAP_TAG_STRING: i32 = 1;
/// ヒープオブジェクトタグ: レコード
pub const HEAP_TAG_RECORD: i32 = 2;
/// ヒープオブジェクトタグ: ADT
pub const HEAP_TAG_ADT: i32 = 3;
/// ヒープオブジェクトタグ: クロージャ
pub const HEAP_TAG_CLOSURE: i32 = 4;
/// ヒープオブジェクトタグ: ベクタ
pub const HEAP_TAG_VECTOR: i32 = 5;
/// ヒープオブジェクトタグ: ハッシュマップ
pub const HEAP_TAG_HASHMAP: i32 = 6;
/// ヒープオブジェクトタグ: Ref (可変参照)
pub const HEAP_TAG_REF: i32 = 7;

/// i32 アドレスをタグ付きポインタ (i64) に変換する IR 命令列を生成
/// 最上位ビットを 1 にセット: addr | (1 << 63)
/// スタック: [addr: i32] -> [tagged_ptr: i64]
pub(crate) fn emit_tag_pointer(body: &mut Vec<crate::Instruction>, _addr_local: u32) {
    use crate::Instruction;
    // スタックトップの i32 アドレスを i64 に拡張してタグ付け
    body.push(Instruction::I64ExtendI32U);
    body.push(Instruction::I64Const(1i64 << 63));
    body.push(Instruction::I64Add); // OR の代わりに ADD (最上位ビットが 0 なので等価)
}

/// タグ付きポインタ (i64) から i32 アドレスを取り出す IR 命令列を生成
/// 下位 32 ビットを取得: ptr as i32
/// スタック: [tagged_ptr: i64] -> [addr: i32]
pub(crate) fn emit_untag_pointer(body: &mut Vec<crate::Instruction>) {
    use crate::Instruction;
    body.push(Instruction::I32WrapI64);
}

/// ヒープオブジェクトヘッダ [tag: i32, size: i32] を書き込む IR 命令列を生成
/// スタック: [addr: i32] -> [] (アドレスは消費される、呼び出し側で保存が必要)
/// addr+0 に tag、addr+4 に size を書き込む
pub(crate) fn emit_write_heap_header(
    body: &mut Vec<crate::Instruction>,
    tag: i32,
    size: i32,
) {
    use crate::Instruction;
    // I32Store はスタックから [addr, value] を消費する
    // tag を書き込み: mem[addr+0] = tag
    body.push(Instruction::I32Const(tag));
    body.push(Instruction::I32Store { offset: 0 });
    // size を書き込み: mem[addr+4] = size
    // 注意: addr は I32Store で消費済み。呼び出し側が LocalTee/LocalGet でアドレスを再供給する
    body.push(Instruction::I32Const(size));
    body.push(Instruction::I32Store { offset: 4 });
}

/// TypeExpr -> IR 型（簡易変換）
pub(crate) fn type_expr_to_ir(ty: &lsharp_syntax::ast::TypeExpr) -> IrType {
    match ty {
        TypeExpr::Named(_, name) => match name.as_str() {
            "Int" => IrType::I64,
            "Float" => IrType::F64,
            "Bool" => IrType::I64,
            "Unit" => IrType::I64,
            "String" => IrType::I64,
            _ => IrType::I64,
        },
        _ => IrType::I64,
    }
}
