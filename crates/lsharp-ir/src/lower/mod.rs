//! Typed AST -> IR 変換 (Lowering)

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;
use lsharp_types::infer::ExprTypeKey;
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
    Unsupported { msg: String, span: Option<Span> },

    #[error("未定義の関数: {name}")]
    UndefinedFunction { name: String, span: Option<Span> },
}

/// Lowering が値表現を選ぶ backend。
///
/// `Linear` は既存の tagged pointer / linear-memory 表現を維持し、
/// `WasmGc` はレコード値を `IrType::Ref` として WasmGC emitter へ渡す。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LowerBackend {
    Linear,
    WasmGc,
}

impl LowerError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "LS3001",
            Self::UndefinedFunction { .. } => "LS3002",
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Unsupported { span, .. } | Self::UndefinedFunction { span, .. } => *span,
        }
    }
}

/// Lowering コンテキスト
pub struct Lower {
    /// 値表現を選択する backend。
    pub(crate) backend: LowerBackend,
    /// 関数名 -> 関数インデックスのマッピング
    pub(crate) func_indices: HashMap<String, u32>,
    /// import 関数の数（ユーザー関数のインデックスオフセット）
    pub(crate) import_count: u32,
    /// 型推論結果
    pub(crate) type_results: HashMap<String, Type>,
    /// 式レベルの型推論結果
    pub(crate) expr_type_results: HashMap<ExprTypeKey, Type>,
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
    /// ADT バリアント名 -> (GC 型インデックス, タグ値)
    pub(crate) adt_variant_indices: HashMap<String, (u32, i32)>,
    /// ADT 型名 -> バリアント情報リスト [(name, gc_idx, tag, field_count)]
    pub(crate) adt_type_info: HashMap<String, Vec<(String, u32, i32, usize)>>,
    /// ADT 型名 -> WasmGC struct 型インデックス
    pub(crate) adt_type_indices: HashMap<String, u32>,
    /// ADT バリアント名 -> payload field の WasmGC 型
    pub(crate) adt_variant_field_types: HashMap<String, Vec<IrType>>,
    /// ADT バリアント名 -> 共通 GC struct 内の payload field offset
    pub(crate) adt_variant_field_offsets: HashMap<String, Vec<u32>>,
    /// ADT バリアント名 -> payload field のソース型名
    pub(crate) adt_variant_field_type_names: HashMap<String, Vec<Option<String>>>,
    /// ADT 型名 -> 共通 payload slot の WasmGC 型
    pub(crate) adt_slot_types: HashMap<String, Vec<IrType>>,
    /// WasmGC ADT の表現上の未対応理由
    pub(crate) adt_field_errors: Vec<String>,
    /// 文字列定数データ [(label, bytes)]
    pub(crate) string_data: Vec<(String, Vec<u8>)>,
    /// 次の文字列データオフセット
    pub(crate) string_offset: u32,
    /// Computation Builder 情報（ビルダー名 -> (bind関数名, return関数名)）
    pub(crate) computation_builders: HashMap<String, (String, String)>,
    /// Lambda Lifting: リフトされた関数のリスト
    pub(crate) lifted_functions: Vec<crate::Function>,
    /// Lambda Lifting: 一意な Lambda 名生成用カウンター
    pub(crate) lambda_counter: u32,
    /// Lambda Lifting: リフトされた関数名 -> 関数インデックスのマッピング
    pub(crate) lifted_func_indices: HashMap<String, u32>,
    /// Lambda Lifting: 次に割り当てる関数インデックス
    pub(crate) next_func_idx: u32,
    /// lower_program 後半で追加登録する補助関数のインデックスカーソル
    pub(crate) late_func_idx: u32,
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
        Self::with_backend(LowerBackend::Linear)
    }

    /// backend を明示して lowering コンテキストを作成する。
    pub fn with_backend(backend: LowerBackend) -> Self {
        Self {
            backend,
            func_indices: HashMap::new(),
            import_count: 0,
            type_results: HashMap::new(),
            expr_type_results: HashMap::new(),
            record_type_indices: HashMap::new(),
            record_fields: HashMap::new(),
            gc_types: Vec::new(),
            trait_method_impls: HashMap::new(),
            trait_method_names: HashMap::new(),
            adt_variant_indices: HashMap::new(),
            adt_type_info: HashMap::new(),
            adt_type_indices: HashMap::new(),
            adt_variant_field_types: HashMap::new(),
            adt_variant_field_offsets: HashMap::new(),
            adt_variant_field_type_names: HashMap::new(),
            adt_slot_types: HashMap::new(),
            adt_field_errors: Vec::new(),
            string_data: Vec::new(),
            string_offset: 512, // 文字列データの開始位置（メモリ先頭は数値変換バッファ用）
            computation_builders: HashMap::new(),
            lifted_functions: Vec::new(),
            lambda_counter: 0,
            lifted_func_indices: HashMap::new(),
            next_func_idx: 0,
            late_func_idx: 0,
        }
    }

    pub fn backend(&self) -> LowerBackend {
        self.backend
    }

    /// 一意な Lambda 関数名を生成
    pub(crate) fn fresh_lambda_name(&mut self) -> String {
        let id = self.lambda_counter;
        self.lambda_counter += 1;
        format!("__lambda_{id}")
    }

    fn reset_state(&mut self) {
        self.func_indices.clear();
        self.import_count = 0;
        self.type_results.clear();
        self.expr_type_results.clear();
        self.record_type_indices.clear();
        self.record_fields.clear();
        self.gc_types.clear();
        self.trait_method_impls.clear();
        self.trait_method_names.clear();
        self.adt_variant_indices.clear();
        self.adt_type_info.clear();
        self.adt_type_indices.clear();
        self.adt_variant_field_types.clear();
        self.adt_variant_field_offsets.clear();
        self.adt_variant_field_type_names.clear();
        self.adt_slot_types.clear();
        self.adt_field_errors.clear();
        self.string_data.clear();
        self.string_offset = 512;
        self.computation_builders.clear();
        self.lifted_functions.clear();
        self.lambda_counter = 0;
        self.lifted_func_indices.clear();
        self.next_func_idx = 0;
        self.late_func_idx = 0;
    }

    pub(crate) fn prepare_program_state(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
    ) {
        self.reset_state();

        // 型推論結果を保存
        for (name, scheme) in type_results {
            self.type_results.insert(name.clone(), scheme.ty.clone());
        }

        // レコード型定義を GC 型として登録
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                let gc_idx = self.gc_types.len() as u32;
                self.record_type_indices.insert(name.clone(), gc_idx);

                let gc_fields: Vec<GcField> = fields
                    .iter()
                    .map(|(fname, ftype)| GcField {
                        name: fname.clone(),
                        ty: self.type_expr_to_ir(ftype),
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

        // WasmGC の ADT struct index を先に予約する。payload が別の ADT を参照していても、
        // 宣言順に依存せず concrete reference type を解決できるようにする。
        if self.backend == LowerBackend::WasmGc {
            for decl in &program.decls {
                if let Decl::TypeDef { name, .. } = unwrap_private(decl) {
                    let gc_idx = self.gc_types.len() as u32;
                    self.adt_type_indices.insert(name.clone(), gc_idx);
                    self.gc_types.push(GcTypeDef {
                        name: name.clone(),
                        kind: GcTypeKind::Struct(Vec::new()),
                    });
                }
            }
        }

        // ADT 型定義を WasmGC struct として登録する。
        // field 0 は variant tag、残りは variant 間で共有する typed payload slot とする。
        for decl in &program.decls {
            if let Decl::TypeDef { name, variants, .. } = unwrap_private(decl) {
                if self.backend != LowerBackend::WasmGc {
                    let variant_infos = variants
                        .iter()
                        .enumerate()
                        .map(|(tag, variant)| {
                            (variant.name.clone(), 0, tag as i32, variant.fields.len())
                        })
                        .collect();
                    for (tag, variant) in variants.iter().enumerate() {
                        self.adt_variant_indices
                            .insert(variant.name.clone(), (0, tag as i32));
                    }
                    self.adt_type_info.insert(name.clone(), variant_infos);
                    continue;
                }
                let gc_idx = self.adt_type_indices.get(name).copied().unwrap_or(0);
                let mut slot_types = Vec::new();
                for variant in variants {
                    let mut field_types = Vec::with_capacity(variant.fields.len());
                    let mut field_type_names = Vec::with_capacity(variant.fields.len());
                    for field in &variant.fields {
                        if let lsharp_syntax::ast::TypeExpr::App(_, head, _) = field
                            && let lsharp_syntax::ast::TypeExpr::Named(_, field_type_name) =
                                head.as_ref()
                            && field_type_name == name
                        {
                            self.adt_field_errors.push(format!(
                                "WasmGC ADT の自己参照 payload は現在未対応です: {name}::{}",
                                variant.name
                            ));
                        }
                        let Some(field_type) = self.wasm_gc_adt_field_type(field) else {
                            self.adt_field_errors.push(format!(
                                "WasmGC ADT payload の型を解決できません: {}::{}",
                                name, variant.name
                            ));
                            continue;
                        };
                        field_types.push(field_type);
                        field_type_names.push(match field {
                            lsharp_syntax::ast::TypeExpr::Named(_, name) => Some(name.clone()),
                            lsharp_syntax::ast::TypeExpr::App(_, head, _) => {
                                if let lsharp_syntax::ast::TypeExpr::Named(_, name) = head.as_ref()
                                {
                                    Some(name.clone())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        });
                    }
                    let field_offsets = field_types
                        .iter()
                        .map(|field_type| {
                            let offset = slot_types.len() as u32;
                            slot_types.push(*field_type);
                            offset
                        })
                        .collect::<Vec<_>>();
                    self.adt_variant_field_types
                        .insert(variant.name.clone(), field_types.clone());
                    self.adt_variant_field_offsets
                        .insert(variant.name.clone(), field_offsets);
                    self.adt_variant_field_type_names
                        .insert(variant.name.clone(), field_type_names);
                }

                self.adt_slot_types.insert(name.clone(), slot_types.clone());

                if self.backend == LowerBackend::WasmGc {
                    let mut gc_fields = Vec::with_capacity(slot_types.len() + 1);
                    gc_fields.push(GcField {
                        name: "tag".to_string(),
                        ty: IrType::I64,
                        mutable: false,
                    });
                    gc_fields.extend(slot_types.iter().enumerate().map(
                        |(field_idx, field_type)| GcField {
                            name: format!("field_{field_idx}"),
                            ty: *field_type,
                            mutable: false,
                        },
                    ));
                    self.gc_types[gc_idx as usize] = GcTypeDef {
                        name: name.clone(),
                        kind: GcTypeKind::Struct(gc_fields),
                    };
                }

                let mut variant_infos = Vec::new();
                for (tag, variant) in variants.iter().enumerate() {
                    let tag_val = tag as i32;
                    self.adt_variant_indices
                        .insert(variant.name.clone(), (gc_idx, tag_val));
                    variant_infos.push((
                        variant.name.clone(),
                        gc_idx,
                        tag_val,
                        variant.fields.len(),
                    ));
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
        self.func_indices.insert("__int_to_string".to_string(), 6);
        self.func_indices.insert("read-file".to_string(), 7);
        self.func_indices.insert("write-file".to_string(), 8);
        self.func_indices.insert("file-exists?".to_string(), 9);
        self.func_indices
            .insert("command-line-args".to_string(), 10);
        self.func_indices.insert("command-line-arg".to_string(), 11);
        self.func_indices.insert("read-stdin".to_string(), 12);
        self.func_indices.insert("__fnv1a_hash".to_string(), 13);
        self.func_indices.insert("root_push".to_string(), 14);
        self.func_indices.insert("root_pop".to_string(), 15);
        self.func_indices.insert("root_set".to_string(), 16);
        self.import_count = 17;

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
                    if let Decl::Defn {
                        name: method_name, ..
                    } = unwrap_private(method_decl)
                    {
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
            if let Decl::ComputationBuilder {
                name,
                bind_fn,
                return_fn,
                ..
            } = unwrap_private(decl)
            {
                self.computation_builders
                    .insert(name.clone(), (bind_fn.clone(), return_fn.clone()));
            }
        }

        // Lambda Lifting 用の次の関数インデックスを設定
        self.next_func_idx = func_idx;
        self.late_func_idx = func_idx;
    }

    pub(crate) fn lower_defn_functions(
        &mut self,
        program: &Program,
    ) -> Result<Vec<crate::Function>, LowerError> {
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
        Ok(functions)
    }

    pub(crate) fn lower_field_accessors(&self, program: &Program) -> Vec<crate::Function> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::RecordDef { name, fields, .. } = unwrap_private(decl) {
                for (field_idx, (fname, ftype)) in fields.iter().enumerate() {
                    let accessor =
                        self.generate_field_accessor(name, fname, field_idx as u32, ftype);
                    functions.push(accessor);
                }
            }
        }
        functions
    }

    pub(crate) fn lower_trait_impl_functions(
        &mut self,
        program: &Program,
    ) -> Result<Vec<crate::Function>, LowerError> {
        let mut functions = Vec::new();
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
        Ok(functions)
    }

    pub(crate) fn lower_constraint_functions(&mut self, program: &Program) -> Vec<crate::Function> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::TypeConstrained {
                name, constraints, ..
            } = unwrap_private(decl)
            {
                // Name.new: (-> BaseType BaseType) -- 制約チェック付き
                let check_func = self.generate_constraint_check(name, constraints);
                // 関数インデックスを登録
                let check_name = format!("{name}.new");
                if !self.func_indices.contains_key(&check_name) {
                    self.func_indices
                        .insert(check_name.clone(), self.late_func_idx);
                    self.late_func_idx += 1;
                }
                functions.push(check_func);

                // Name.valid?: (-> BaseType Bool) -- 検証のみ（トラップしない）
                let valid_func = self.generate_constraint_valid(name, constraints);
                let valid_name = format!("{name}.valid?");
                if !self.func_indices.contains_key(&valid_name) {
                    self.func_indices
                        .insert(valid_name.clone(), self.late_func_idx);
                    self.late_func_idx += 1;
                }
                functions.push(valid_func);
            }
        }
        functions
    }

    pub(crate) fn lower_adt_constructors(&self, program: &Program) -> Vec<crate::Function> {
        let mut functions = Vec::new();
        for decl in &program.decls {
            if let Decl::TypeDef { name, variants, .. } = unwrap_private(decl) {
                let gc_type_idx = self.adt_type_indices.get(name).copied().unwrap_or(0);
                let slot_types = self.adt_slot_types.get(name).cloned().unwrap_or_default();
                for variant in variants {
                    if let Some(&(_, tag_val)) = self.adt_variant_indices.get(&variant.name) {
                        let field_types = if self.backend == LowerBackend::WasmGc {
                            self.adt_variant_field_types
                                .get(&variant.name)
                                .cloned()
                                .unwrap_or_default()
                        } else {
                            vec![IrType::I64; variant.fields.len()]
                        };
                        let constructor_slot_types = if self.backend == LowerBackend::WasmGc {
                            slot_types.clone()
                        } else {
                            vec![IrType::I64; variant.fields.len()]
                        };
                        let field_offsets = if self.backend == LowerBackend::WasmGc {
                            self.adt_variant_field_offsets
                                .get(&variant.name)
                                .cloned()
                                .unwrap_or_default()
                        } else {
                            Vec::new()
                        };
                        let ctor = self.generate_adt_constructor(
                            &variant.name,
                            gc_type_idx,
                            tag_val,
                            &field_types,
                            &constructor_slot_types,
                            &field_offsets,
                        );
                        functions.push(ctor);
                    }
                }
            }
        }
        functions
    }

    pub(crate) fn clone_string_data_from(&self, start: usize) -> Vec<(String, Vec<u8>)> {
        self.string_data[start..].to_vec()
    }

    pub(crate) fn gc_types_for_program(&self, program: &Program) -> Vec<GcTypeDef> {
        let mut gc_types = Vec::new();
        for decl in &program.decls {
            if let Decl::RecordDef { name, .. } = unwrap_private(decl)
                && let Some(&gc_idx) = self.record_type_indices.get(name)
            {
                gc_types.push(self.gc_types[gc_idx as usize].clone());
            }
        }
        for decl in &program.decls {
            if let Decl::TypeDef { name, .. } = unwrap_private(decl)
                && let Some(&gc_idx) = self.adt_type_indices.get(name)
            {
                gc_types.push(self.gc_types[gc_idx as usize].clone());
            }
        }
        gc_types
    }

    /// L# 型を選択した値表現の IR 型へ変換する。
    pub(crate) fn ir_type_for_type(&self, ty: &Type) -> IrType {
        if self.backend == LowerBackend::WasmGc {
            if let Type::Record(name, _) = ty
                && let Some(&gc_idx) = self.record_type_indices.get(name)
            {
                return IrType::Ref(gc_idx);
            }
            if let Some(name) = type_to_name(ty)
                && let Some(&gc_idx) = self.adt_type_indices.get(&name)
            {
                return IrType::Ref(gc_idx);
            }
        }
        type_to_ir(ty)
    }

    /// 宣言中の型式を選択した値表現の IR 型へ変換する。
    pub(crate) fn type_expr_to_ir(&self, ty: &lsharp_syntax::ast::TypeExpr) -> IrType {
        if self.backend == LowerBackend::WasmGc
            && let lsharp_syntax::ast::TypeExpr::Named(_, name) = ty
        {
            if let Some(&gc_idx) = self.record_type_indices.get(name) {
                return IrType::Ref(gc_idx);
            }
            if let Some(&gc_idx) = self.adt_type_indices.get(name) {
                return IrType::Ref(gc_idx);
            }
        }
        type_expr_to_ir(ty)
    }

    fn wasm_gc_adt_field_type(&self, ty: &lsharp_syntax::ast::TypeExpr) -> Option<IrType> {
        match ty {
            lsharp_syntax::ast::TypeExpr::Named(_, name) => match name.as_str() {
                "Int" | "Float" | "Bool" | "Unit" => Some(type_expr_to_ir(ty)),
                _ => self
                    .record_type_indices
                    .get(name)
                    .or_else(|| self.adt_type_indices.get(name))
                    .copied()
                    .map(IrType::Ref),
            },
            lsharp_syntax::ast::TypeExpr::App(_, head, _) => {
                let lsharp_syntax::ast::TypeExpr::Named(_, name) = head.as_ref() else {
                    return None;
                };
                self.record_type_indices
                    .get(name)
                    .or_else(|| self.adt_type_indices.get(name))
                    .copied()
                    .map(IrType::Ref)
            }
            _ => None,
        }
    }

    pub(crate) fn ir_type_for_type_name(&self, type_name: &str) -> IrType {
        if self.backend == LowerBackend::WasmGc {
            if let Some(&gc_idx) = self.record_type_indices.get(type_name) {
                return IrType::Ref(gc_idx);
            }
            if let Some(&gc_idx) = self.adt_type_indices.get(type_name) {
                return IrType::Ref(gc_idx);
            }
        }
        IrType::I64
    }

    /// プログラム全体を IR に変換
    pub fn lower_program(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
    ) -> Result<Module, LowerError> {
        let expr_type_results = HashMap::new();
        self.lower_program_with_expr_types(program, type_results, &expr_type_results)
    }

    pub fn lower_program_with_expr_types(
        &mut self,
        program: &Program,
        type_results: &[(String, lsharp_types::types::TypeScheme)],
        expr_type_results: &HashMap<ExprTypeKey, Type>,
    ) -> Result<Module, LowerError> {
        self.prepare_program_state(program, type_results);
        if self.backend == LowerBackend::WasmGc
            && let Some(message) = self.adt_field_errors.first()
        {
            return Err(LowerError::Unsupported {
                msg: message.clone(),
                span: None,
            });
        }
        self.expr_type_results = expr_type_results.clone();

        let mut functions = self.lower_defn_functions(program)?;
        functions.extend(self.lower_field_accessors(program));
        functions.extend(self.lower_trait_impl_functions(program)?);
        functions.extend(self.lower_constraint_functions(program));
        functions.extend(self.lower_adt_constructors(program));

        // Lambda Lifting: リフトされた関数を追加
        let lifted = self.lifted_functions.clone();
        functions.extend(lifted);

        Ok(Module {
            functions,
            gc_types: self.gc_types.clone(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: self.string_data.clone(),
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
    pub(crate) function_name: String,
    pub(crate) type_scope_key: String,
    pub(crate) instructions: Vec<crate::Instruction>,
    pub(crate) locals_map: HashMap<String, u32>,
    pub(crate) local_type_names: HashMap<String, String>,
    pub(crate) param_count: u32,
    pub(crate) next_local: u32,
    /// Wasm local index ごとの IR 型（param を除く extra local の型生成にも使う）。
    pub(crate) local_types: Vec<IrType>,
}

impl FuncCtx {
    pub(crate) fn with_type_scope(name: String, type_scope_key: String) -> Self {
        Self {
            function_name: name,
            type_scope_key,
            instructions: Vec::new(),
            locals_map: HashMap::new(),
            local_type_names: HashMap::new(),
            param_count: 0,
            next_local: 0,
            local_types: Vec::new(),
        }
    }

    pub(crate) fn emit(&mut self, instr: crate::Instruction) {
        self.instructions.push(instr);
    }

    pub(crate) fn alloc_local(&mut self, name: String) -> u32 {
        self.alloc_local_typed(name, IrType::I64)
    }

    pub(crate) fn alloc_local_typed(&mut self, name: String, ty: IrType) -> u32 {
        // compiler が使う `_` prefix の一時ローカルは、入れ子の式で同名再利用すると
        // 外側の一時値を内側の lowering が上書きしてしまうため常に fresh にする。
        if name.starts_with('_') {
            let idx = self.next_local;
            self.next_local += 1;
            self.local_types.push(ty);
            return idx;
        }
        if let Some(&idx) = self.locals_map.get(&name) {
            return idx;
        }
        let idx = self.next_local;
        self.locals_map.insert(name, idx);
        self.next_local += 1;
        self.local_types.push(ty);
        idx
    }

    pub(crate) fn alloc_scoped_local_typed(&mut self, name: String, ty: IrType) -> u32 {
        let idx = self.next_local;
        self.locals_map.insert(name, idx);
        self.next_local += 1;
        self.local_types.push(ty);
        idx
    }

    pub(crate) fn restore_local_binding(
        &mut self,
        name: String,
        previous_local: Option<u32>,
        previous_type: Option<String>,
    ) {
        if let Some(idx) = previous_local {
            self.locals_map.insert(name.clone(), idx);
        } else {
            self.locals_map.remove(&name);
        }

        if let Some(type_name) = previous_type {
            self.local_type_names.insert(name, type_name);
        } else {
            self.local_type_names.remove(&name);
        }
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
            | "=="
            | "="
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
            "Bool" => IrType::I64,   // Bool は i64 (0/1)
            "Unit" => IrType::I64,   // Unit も i64 (0)
            "String" => IrType::I64, // MVP: 文字列はポインタ (i64)
            _ => IrType::I64,
        },
        Type::Var(_) => IrType::I64,    // 未解決の型変数はデフォルト i64
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

pub(crate) fn is_heap_like_type_name(type_name: &str) -> bool {
    !matches!(type_name, "Int" | "Float" | "Bool" | "Unit")
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
#[allow(dead_code)]
pub(crate) fn emit_write_heap_header(body: &mut Vec<crate::Instruction>, tag: i32, size: i32) {
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
