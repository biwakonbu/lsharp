//! Typed AST -> IR 変換 (Lowering)

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::types::Type;

use crate::{Function, GcField, GcTypeDef, GcTypeKind, Instruction, IrType, Module};

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
    func_indices: HashMap<String, u32>,
    /// import 関数の数（ユーザー関数のインデックスオフセット）
    import_count: u32,
    /// 型推論結果
    type_results: HashMap<String, Type>,
    /// レコード型名 -> GC 型インデックス
    record_type_indices: HashMap<String, u32>,
    /// レコード型名 -> フィールド名リスト（順序保持）
    record_fields: HashMap<String, Vec<String>>,
    /// GC 型定義のリスト
    gc_types: Vec<GcTypeDef>,
    /// トレイト実装メソッドの解決テーブル
    /// (trait_name, type_name, method_name) -> 関数名
    trait_method_impls: HashMap<(String, String, String), String>,
    /// トレイトメソッド名 -> トレイト名の逆引きテーブル（静的ディスパッチ用）
    /// method_name -> Vec<trait_name>
    trait_method_names: HashMap<String, Vec<String>>,
    /// 制約付き型の登録情報 (ランタイム検証用)
    #[allow(dead_code)]
    constrained_type_checks: HashMap<String, Vec<(String, i64, i64)>>,
    /// ADT バリアント名 -> (GC 型インデックス, タグ値)
    adt_variant_indices: HashMap<String, (u32, i32)>,
    /// ADT 型名 -> バリアント情報リスト [(name, gc_idx, tag, field_count)]
    adt_type_info: HashMap<String, Vec<(String, u32, i32, usize)>>,
    /// 文字列定数データ [(label, bytes)]
    string_data: RefCell<Vec<(String, Vec<u8>)>>,
    /// 次の文字列データオフセット
    string_offset: Cell<u32>,
}

/// Private 宣言を展開して内部の宣言を返す
fn unwrap_private(decl: &Decl) -> &Decl {
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
        }
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

        // ADT 型定義を GC 型として登録
        for decl in &program.decls {
            if let Decl::TypeDef {
                name, variants, ..
            } = unwrap_private(decl)
            {
                let mut variant_infos = Vec::new();
                for (tag, variant) in variants.iter().enumerate() {
                    let gc_idx = self.gc_types.len() as u32;
                    let tag_val = tag as i32;

                    // 各バリアントの struct 型: $tag: i32 + フィールド
                    let mut gc_fields = vec![GcField {
                        name: "$tag".to_string(),
                        ty: IrType::I32,
                        mutable: false,
                    }];
                    for (i, _field_ty) in variant.fields.iter().enumerate() {
                        gc_fields.push(GcField {
                            name: format!("$field{i}"),
                            ty: IrType::I64, // MVP: 全フィールド i64
                            mutable: false,
                        });
                    }

                    self.gc_types.push(GcTypeDef {
                        name: format!("{}.{}", name, variant.name),
                        kind: GcTypeKind::Struct(gc_fields),
                    });

                    self.adt_variant_indices.insert(variant.name.clone(), (gc_idx, tag_val));
                    variant_infos.push((variant.name.clone(), gc_idx, tag_val, variant.fields.len()));
                }
                self.adt_type_info.insert(name.clone(), variant_infos);
            }
        }

        // import 関数を登録 (print = index 0)
        self.func_indices.insert("print".to_string(), 0);
        self.import_count = 1;

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

        Ok(Module {
            functions,
            gc_types: self.gc_types.clone(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: self.string_data.borrow().clone(),
        })
    }

    /// レコード型のフィールドインデックスを解決
    fn resolve_field_index(&self, type_name: &str, field_name: &str) -> Option<u32> {
        if let Some(fields) = self.record_fields.get(type_name) {
            fields.iter().position(|f| f == field_name).map(|i| i as u32)
        } else {
            None
        }
    }

    /// トレイトメソッド呼び出しを静的ディスパッチで解決（P5-6）
    ///
    /// 関数名がトレイトメソッドに該当する場合、第一引数の型情報から
    /// 具体的な実装（マングル名）を解決して関数インデックスを返す。
    fn resolve_trait_dispatch(&self, method_name: &str, args: &[Expr]) -> Option<u32> {
        // メソッド名がトレイトメソッドとして登録されているか確認
        let trait_names = self.trait_method_names.get(method_name)?;

        // 第一引数の型名を推定
        let first_arg_type = if let Some(arg) = args.first() {
            self.infer_expr_type_name(arg)
        } else {
            None
        };

        if let Some(type_name) = first_arg_type {
            // (trait_name, type_name, method_name) でマングル名を検索
            for trait_name in trait_names {
                let key = (trait_name.clone(), type_name.clone(), method_name.to_string());
                if let Some(mangled) = self.trait_method_impls.get(&key) {
                    return self.func_indices.get(mangled).copied();
                }
            }
        }

        // 型が不明な場合、実装が1つだけならそれを使う（一意解決）
        for trait_name in trait_names {
            let matching: Vec<_> = self.trait_method_impls.iter()
                .filter(|((t, _, m), _)| t == trait_name && m == method_name)
                .collect();
            if matching.len() == 1 {
                let (_, mangled) = matching[0];
                return self.func_indices.get(mangled).copied();
            }
        }

        None
    }

    /// 式の型名を推定する（静的ディスパッチ用の簡易推定）
    fn infer_expr_type_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            // リテラルから型を推定
            Expr::Lit(_, Literal::Int(_)) => Some("Int".to_string()),
            Expr::Lit(_, Literal::Float(_)) => Some("Float".to_string()),
            Expr::Lit(_, Literal::Bool(_)) => Some("Bool".to_string()),
            Expr::Lit(_, Literal::String(_)) => Some("String".to_string()),
            Expr::Lit(_, Literal::Unit) => Some("Unit".to_string()),
            // 変数の場合、型推論結果から型名を取得
            Expr::Var(_, name) => {
                if let Some(ty) = self.type_results.get(name) {
                    type_to_name(ty)
                } else {
                    None
                }
            }
            // 型注釈がある場合
            Expr::Ann(_, _, type_expr) => type_expr_to_name(type_expr),
            // レコードリテラルの場合、型名が明示的
            Expr::RecordLit(_, type_name, _) => Some(type_name.clone()),
            // 関数呼び出しの場合、戻り値型を推定
            Expr::App(_, func, _) => {
                if let Expr::Var(_, func_name) = func.as_ref() {
                    if let Some(ty) = self.type_results.get(func_name) {
                        match ty {
                            Type::Fun(_, ret) => type_to_name(ret),
                            _ => None,
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// フィールドアクセサ関数を生成
    fn generate_field_accessor(
        &self,
        type_name: &str,
        field_name: &str,
        field_idx: u32,
        _field_type: &lsharp_syntax::ast::TypeExpr,
    ) -> Function {
        let accessor_name = format!("{type_name}.{field_name}");

        // MVP: レコードはフラットに i64 として扱う
        // 将来的に WasmGC struct.get に変換
        let mut body = Vec::new();
        if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
            body.push(Instruction::LocalGet(0));
            body.push(Instruction::StructGet(gc_type_idx, field_idx));
        } else {
            // フォールバック: 引数をそのまま返す
            body.push(Instruction::LocalGet(0));
        }

        Function {
            name: accessor_name,
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }


    /// ADT コンストラクタ関数を生成 (P1-9: WasmGC)
    ///
    /// 各バリアントに対して struct.new で GC struct を構築する関数を生成。
    /// 例: (Just x) → $tag=0, $field0=x の struct を構築
    fn generate_adt_constructor(
        &self,
        variant_name: &str,
        gc_type_idx: u32,
        tag_val: i32,
        field_count: usize,
    ) -> Function {
        let mut body = Vec::new();

        // $tag をスタックに積む
        body.push(Instruction::I32Const(tag_val));

        // 各フィールドをスタックに積む（引数から取得）
        for i in 0..field_count {
            body.push(Instruction::LocalGet(i as u32));
        }

        // struct.new で構築
        body.push(Instruction::StructNew(gc_type_idx));

        // MVP: GC struct → i64 にキャスト（wasmtime GC 未対応のため）
        // 将来的には ref 型をそのまま返す
        // 現在は i64 0 をフォールバックとして返す
        let mut fallback_body = Vec::new();
        for i in 0..field_count {
            fallback_body.push(Instruction::LocalGet(i as u32));
        }
        // 最後の引数を返す（フィールドが1つの場合はその値、0の場合は 0）
        if field_count > 0 {
            fallback_body.push(Instruction::I64Const(tag_val as i64));
            // タグ値をエンコード: tag << 32 | value（MVP 簡易表現）
        } else {
            fallback_body.push(Instruction::I64Const(tag_val as i64));
        }

        Function {
            name: variant_name.to_string(),
            params: vec![IrType::I64; field_count],
            result: IrType::I64,
            locals: Vec::new(),
            body: fallback_body, // MVP: i64 フォールバック
            is_export: false,
        }
    }

    /// 制約付き型のスマートコンストラクタ (Name.new) を生成
    /// 制約を満たさない場合は unreachable (トラップ) する
    fn generate_constraint_check(
        &self,
        type_name: &str,
        constraints: &[Constraint],
    ) -> Function {
        let func_name = format!("{type_name}.new");
        let mut body = Vec::new();

        for constraint in constraints {
            match constraint {
                Constraint::Gte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                    }
                }
                Constraint::Lte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                    }
                }
                Constraint::Range(lo_expr, hi_expr) => {
                    if let (Expr::Lit(_, Literal::Int(lo)), Expr::Lit(_, Literal::Int(hi))) =
                        (lo_expr, hi_expr)
                    {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*lo));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*hi));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I32Eqz);
                        body.push(Instruction::If(IrType::I64));
                        body.push(Instruction::Unreachable);
                        body.push(Instruction::Else);
                        body.push(Instruction::I64Const(0));
                        body.push(Instruction::End);
                        body.push(Instruction::Drop);
                    }
                }
                _ => {}
            }
        }

        body.push(Instruction::LocalGet(0));

        Function {
            name: func_name,
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }

    /// 制約付き型の検証関数 (Name.valid?) を生成
    fn generate_constraint_valid(
        &self,
        type_name: &str,
        constraints: &[Constraint],
    ) -> Function {
        let func_name = format!("{type_name}.valid?");
        let mut body = Vec::new();

        body.push(Instruction::I64Const(1));

        for constraint in constraints {
            match constraint {
                Constraint::Gte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                    }
                }
                Constraint::Lte(expr) => {
                    if let Expr::Lit(_, Literal::Int(threshold)) = expr {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*threshold));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                    }
                }
                Constraint::Range(lo_expr, hi_expr) => {
                    if let (Expr::Lit(_, Literal::Int(lo)), Expr::Lit(_, Literal::Int(hi))) =
                        (lo_expr, hi_expr)
                    {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*lo));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*hi));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I64ExtendI32S);
                        body.push(Instruction::I32WrapI64);
                        body.push(Instruction::I32And);
                        body.push(Instruction::I64ExtendI32S);
                    }
                }
                _ => {}
            }
        }

        Function {
            name: func_name,
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }

    /// 関数を IR に変換
    fn lower_function(
        &self,
        name: &str,
        params: &[Param],
        body: &Expr,
    ) -> Result<Function, LowerError> {
        let mut ctx = FuncCtx::new(name.to_string());

        // パラメータをローカル変数として登録
        for param in params {
            let idx = ctx.next_local;
            ctx.locals_map.insert(param.name.clone(), idx);
            ctx.param_count += 1;
            ctx.next_local += 1;
        }

        // 本体を変換
        self.lower_expr(&mut ctx, body)?;

        // 関数型を推論結果から取得
        let (param_types, result_type) = if let Some(ty) = self.type_results.get(name) {
            match ty {
                Type::Fun(params, ret) => {
                    let p: Vec<IrType> = params.iter().map(|t| type_to_ir(t)).collect();
                    let r = type_to_ir(ret);
                    (p, r)
                }
                _ => (Vec::new(), type_to_ir(ty)),
            }
        } else {
            let p = vec![IrType::I64; params.len()];
            (p, IrType::I64)
        };

        // ローカル変数（パラメータ以外）
        let extra_locals = vec![IrType::I64; (ctx.next_local - ctx.param_count) as usize];

        Ok(Function {
            name: name.to_string(),
            params: param_types,
            result: result_type,
            locals: extra_locals,
            body: ctx.instructions,
            is_export: name == "main",
        })
    }

    /// 式を IR 命令に変換（スタックマシン方式）
    fn lower_expr(&self, ctx: &mut FuncCtx, expr: &Expr) -> Result<(), LowerError> {
        match expr {
            Expr::Lit(_, lit) => match lit {
                Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
                Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
                Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
                Literal::String(s) => {
                    // 文字列リテラル: データセクションに格納し、(offset << 32 | len) でエンコード
                    let bytes = s.as_bytes().to_vec();
                    let len = bytes.len() as u32;
                    let offset = self.string_offset.get();
                    let label = format!("$str{}", self.string_data.borrow().len());
                    self.string_data.borrow_mut().push((label, bytes));
                    self.string_offset.set(offset + len);
                    // offset << 32 | len として i64 にパック
                    let packed = ((offset as i64) << 32) | (len as i64);
                    ctx.emit(Instruction::I64Const(packed));
                }
                Literal::Unit => ctx.emit(Instruction::I64Const(0)),
            },

            Expr::Var(_, name) => {
                if let Some(&idx) = ctx.locals_map.get(name) {
                    ctx.emit(Instruction::LocalGet(idx));
                } else if let Some(&func_idx) = self.func_indices.get(name) {
                    // 引数なし ADT コンストラクタ（または引数なし関数）を呼び出し
                    ctx.emit(Instruction::Call(func_idx));
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                    });
                }
            }

            Expr::If(_, cond, then, else_) => {
                // 条件式
                self.lower_expr(ctx, cond)?;
                // Bool (i64) -> i32 に変換
                ctx.emit(Instruction::I32WrapI64);
                // if-then-else
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, then)?;
                ctx.emit(Instruction::Else);
                self.lower_expr(ctx, else_)?;
                ctx.emit(Instruction::End);
            }

            Expr::Let(_, bindings, body) => {
                for (pat, val) in bindings {
                    self.lower_expr(ctx, val)?;
                    match pat {
                        Pattern::Var(_, name) => {
                            let idx = ctx.alloc_local(name.clone());
                            ctx.emit(Instruction::LocalSet(idx));
                        }
                        Pattern::Wildcard(_) => {
                            ctx.emit(Instruction::Drop);
                        }
                        _ => {
                            // MVP: 複雑なパターンは未サポート
                            let idx = ctx.alloc_local("_pat".to_string());
                            ctx.emit(Instruction::LocalSet(idx));
                        }
                    }
                }
                self.lower_expr(ctx, body)?;
            }

            Expr::App(_, func, args) => {
                match func.as_ref() {
                    // and/or 論理演算子（i64 -> i32 変換が必要）
                    Expr::Var(_, op) if (op == "and" || op == "or") && args.len() == 2 => {
                        // 左オペランド: i64 -> i32
                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::I32WrapI64);
                        // 右オペランド: i64 -> i32
                        self.lower_expr(ctx, &args[1])?;
                        ctx.emit(Instruction::I32WrapI64);
                        // i32 レベルで and/or
                        if op == "and" {
                            ctx.emit(Instruction::I32And);
                        } else {
                            ctx.emit(Instruction::I32Or);
                        }
                        // 結果を i64 に拡張
                        ctx.emit(Instruction::I64ExtendI32S);
                    }
                    // 組み込み二項演算子
                    Expr::Var(_, op) if is_builtin_binop(op) && args.len() == 2 => {
                        self.lower_expr(ctx, &args[0])?;
                        self.lower_expr(ctx, &args[1])?;
                        self.emit_binop(ctx, op);
                    }
                    // not 演算子
                    Expr::Var(_, op) if op == "not" && args.len() == 1 => {
                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::I64Const(0));
                        ctx.emit(Instruction::I64Eq);
                        ctx.emit(Instruction::I64ExtendI32S);
                    }
                    // print 関数
                    Expr::Var(_, name) if name == "print" => {
                        if let Some(arg) = args.first() {
                            self.lower_expr(ctx, arg)?;
                        }
                        let idx = self.func_indices["print"];
                        ctx.emit(Instruction::Call(idx));
                        // print は Unit を返す
                        ctx.emit(Instruction::I64Const(0));
                    }
                    // TypeName.field アクセサ呼び出し
                    Expr::Var(_, name) if name.contains('.') && name.starts_with(|c: char| c.is_ascii_uppercase()) => {
                        // 引数（レコード）を評価
                        for arg in args {
                            self.lower_expr(ctx, arg)?;
                        }
                        if let Some(&idx) = self.func_indices.get(name.as_str()) {
                            ctx.emit(Instruction::Call(idx));
                        } else {
                            return Err(LowerError::UndefinedFunction {
                                name: name.clone(),
                            });
                        }
                    }
                    // ユーザー定義関数呼び出し（トレイト静的ディスパッチ対応）
                    Expr::Var(_, name) => {
                        // 引数を評価
                        for arg in args {
                            self.lower_expr(ctx, arg)?;
                        }
                        if let Some(&idx) = self.func_indices.get(name.as_str()) {
                            ctx.emit(Instruction::Call(idx));
                        } else if let Some(idx) = self.resolve_trait_dispatch(name, args) {
                            // P5-6: トレイトメソッドの静的ディスパッチ自動解決
                            ctx.emit(Instruction::Call(idx));
                        } else {
                            return Err(LowerError::UndefinedFunction {
                                name: name.clone(),
                            });
                        }
                    }
                    _ => {
                        return Err(LowerError::Unsupported {
                            msg: "間接的な関数呼び出し".to_string(),
                        });
                    }
                }
            }

            Expr::Match(_, scrutinee, arms) => {
                // MVP: 簡易パターンマッチ（ADT なし、リテラル/変数のみ）
                // scrutinee を評価してローカルに保存
                self.lower_expr(ctx, scrutinee)?;
                let scrut_local = ctx.alloc_local("_match".to_string());
                ctx.emit(Instruction::LocalSet(scrut_local));

                // ネストした if-else チェインで変換
                self.lower_match_arms(ctx, scrut_local, arms, 0)?;
            }

            Expr::Do(_, exprs) => {
                for (i, expr) in exprs.iter().enumerate() {
                    self.lower_expr(ctx, expr)?;
                    // 最後の式以外は結果を捨てる
                    if i < exprs.len() - 1 {
                        ctx.emit(Instruction::Drop);
                    }
                }
                if exprs.is_empty() {
                    ctx.emit(Instruction::I64Const(0)); // unit
                }
            }

            Expr::Lambda(_, _, _) => {
                // MVP: ラムダ（クロージャ）は未サポート
                return Err(LowerError::Unsupported {
                    msg: "ラムダ式（クロージャ）".to_string(),
                });
            }

            Expr::Ann(_, expr, _) => {
                // 型注釈は無視して中身を変換
                self.lower_expr(ctx, expr)?;
            }

            Expr::RecordLit(_, type_name, fields) => {
                if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
                    // レコード定義のフィールド順序に従って値をスタックに積む
                    if let Some(field_order) = self.record_fields.get(type_name) {
                        let field_map: HashMap<&str, &Expr> = fields
                            .iter()
                            .map(|(n, e)| (n.as_str(), e))
                            .collect();
                        for field_name in field_order {
                            if let Some(expr) = field_map.get(field_name.as_str()) {
                                self.lower_expr(ctx, expr)?;
                            } else {
                                // フィールドが見つからない場合はデフォルト値
                                ctx.emit(Instruction::I64Const(0));
                            }
                        }
                    } else {
                        // フィールド順序不明の場合は指定順に積む
                        for (_, field_expr) in fields {
                            self.lower_expr(ctx, field_expr)?;
                        }
                    }
                    ctx.emit(Instruction::StructNew(gc_type_idx));
                } else {
                    // GC 型が見つからない場合はフォールバック
                    if let Some((_, first_field)) = fields.first() {
                        self.lower_expr(ctx, first_field)?;
                    } else {
                        ctx.emit(Instruction::I64Const(0));
                    }
                }
            }

            Expr::FieldAccess(_, expr, field_name) => {
                // 式を評価してスタックにレコード値を積む
                self.lower_expr(ctx, expr)?;

                // 型名からフィールドインデックスを解決
                // TypeName.field 形式のフィールドアクセス
                let mut resolved = false;
                for (type_name, fields) in &self.record_fields {
                    if let Some(field_idx) = fields.iter().position(|f| f == field_name) {
                        if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
                            ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                            resolved = true;
                            break;
                        }
                    }
                }

                if !resolved {
                    // フォールバック: 式をそのまま返す（既にスタック上にある）
                }
            }

            Expr::RecordUpdate(_, base, update_fields) => {
                // ベースレコードを評価してローカルに保存
                self.lower_expr(ctx, base)?;
                let base_local = ctx.alloc_local("_record_base".to_string());
                ctx.emit(Instruction::LocalSet(base_local));

                // 型名を推定（最初に見つかるレコード型を使用）
                // TODO: 型推論結果から正確な型名を取得
                let mut found_type = None;
                for (type_name, fields) in &self.record_fields {
                    // 更新フィールドが全てこの型に含まれるか
                    let all_match = update_fields.iter().all(|(n, _)| fields.contains(n));
                    if all_match {
                        found_type = Some((type_name.clone(), fields.clone()));
                        break;
                    }
                }

                if let Some((type_name, field_order)) = found_type {
                    if let Some(&gc_type_idx) = self.record_type_indices.get(&type_name) {
                        let update_map: HashMap<&str, &Expr> = update_fields
                            .iter()
                            .map(|(n, e)| (n.as_str(), e))
                            .collect();
                        // 各フィールドについて、更新値があればそれを、なければベースから取得
                        for (field_idx, field_name) in field_order.iter().enumerate() {
                            if let Some(expr) = update_map.get(field_name.as_str()) {
                                self.lower_expr(ctx, expr)?;
                            } else {
                                ctx.emit(Instruction::LocalGet(base_local));
                                ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                            }
                        }
                        ctx.emit(Instruction::StructNew(gc_type_idx));
                    } else {
                        ctx.emit(Instruction::LocalGet(base_local));
                    }
                } else {
                    // フォールバック: ベースをそのまま返す
                    ctx.emit(Instruction::LocalGet(base_local));
                }
            }
            Expr::Computation(_, _builder, steps) => {
                // Computation Expression: 各ステップを順次評価
                // TODO: bind/return への脱糖と適切なモナド変換
                for step in steps {
                    match step {
                        ComputationStep::LetBang(_, _, expr) => self.lower_expr(ctx, expr)?,
                        ComputationStep::DoBang(_, expr) => self.lower_expr(ctx, expr)?,
                        ComputationStep::Return(_, expr) => self.lower_expr(ctx, expr)?,
                        ComputationStep::Expr(expr) => self.lower_expr(ctx, expr)?,
                    }
                }
            }
        }

        Ok(())
    }

    /// match の腕を if-else チェインに変換
    fn lower_match_arms(
        &self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        arms: &[MatchArm],
        idx: usize,
    ) -> Result<(), LowerError> {
        if idx >= arms.len() {
            // 到達不能（網羅性チェック済みの前提）
            ctx.emit(Instruction::Unreachable);
            return Ok(());
        }

        let arm = &arms[idx];

        match &arm.pattern {
            // ワイルドカードや変数パターンは常にマッチ
            Pattern::Wildcard(_) => {
                self.lower_expr(ctx, &arm.body)?;
            }
            Pattern::Var(_, name) => {
                // scrutinee を変数に束縛
                ctx.emit(Instruction::LocalGet(scrut_local));
                let var_local = ctx.alloc_local(name.clone());
                ctx.emit(Instruction::LocalSet(var_local));
                self.lower_expr(ctx, &arm.body)?;
            }
            Pattern::Lit(_, Literal::Int(n)) => {
                // scrutinee == n なら本体を実行
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(*n));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, &arm.body)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Lit(_, Literal::Bool(b)) => {
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 }));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, &arm.body)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Constructor(_, name, sub_pats) if sub_pats.is_empty() => {
                // 引数なしコンストラクタ: タグ比較
                let _ = name;
                if idx == arms.len() - 1 {
                    // 最後の腕はデフォルトとして扱う
                    self.lower_expr(ctx, &arm.body)?;
                } else {
                    // 次の腕と if-else
                    ctx.emit(Instruction::If(IrType::I64));
                    self.lower_expr(ctx, &arm.body)?;
                    ctx.emit(Instruction::Else);
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                    ctx.emit(Instruction::End);
                }
            }
            Pattern::Constructor(_, _name, sub_pats) => {
                // MVP: 引数付きコンストラクタは変数パターンに退化
                if let Some(Pattern::Var(_, var_name)) = sub_pats.first() {
                    ctx.emit(Instruction::LocalGet(scrut_local));
                    let var_local = ctx.alloc_local(var_name.clone());
                    ctx.emit(Instruction::LocalSet(var_local));
                }
                if idx == arms.len() - 1 {
                    self.lower_expr(ctx, &arm.body)?;
                } else {
                    self.lower_expr(ctx, &arm.body)?;
                }
            }
            Pattern::RecordPat(_, type_name, field_pats) => {
                // レコードパターン: StructGet でフィールドを抽出
                for (field_name, field_pat) in field_pats {
                    if let Pattern::Var(_, var_name) = field_pat {
                        // フィールドインデックスを解決
                        let field_idx = self.resolve_field_index(type_name, field_name);
                        let gc_type_idx = self.record_type_indices.get(type_name.as_str()).copied();

                        ctx.emit(Instruction::LocalGet(scrut_local));
                        if let (Some(gc_idx), Some(f_idx)) = (gc_type_idx, field_idx) {
                            // GC 型が登録されている場合は StructGet を使用
                            ctx.emit(Instruction::StructGet(gc_idx, f_idx));
                        }
                        // StructGet 結果（または scrutinee 自体）をローカルに格納
                        let var_local = ctx.alloc_local(var_name.clone());
                        ctx.emit(Instruction::LocalSet(var_local));
                    }
                }
                self.lower_expr(ctx, &arm.body)?;
            }
            _ => {
                return Err(LowerError::Unsupported {
                    msg: format!("パターン: {:?}", arm.pattern),
                });
            }
        }

        Ok(())
    }

    /// 二項演算子の IR 命令を出力
    fn emit_binop(&self, ctx: &mut FuncCtx, op: &str) {
        match op {
            "+" => ctx.emit(Instruction::I64Add),
            "-" => ctx.emit(Instruction::I64Sub),
            "*" => ctx.emit(Instruction::I64Mul),
            "/" => ctx.emit(Instruction::I64Div),
            "%" => ctx.emit(Instruction::I64Rem),
            "+." => ctx.emit(Instruction::F64Add),
            "-." => ctx.emit(Instruction::F64Sub),
            "*." => ctx.emit(Instruction::F64Mul),
            "/." => ctx.emit(Instruction::F64Div),
            "==" | "=" => {
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "!=" => {
                ctx.emit(Instruction::I64Ne);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "<" => {
                ctx.emit(Instruction::I64LtS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            ">" => {
                ctx.emit(Instruction::I64GtS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "<=" => {
                ctx.emit(Instruction::I64LeS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            ">=" => {
                ctx.emit(Instruction::I64GeS);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "and" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32And);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            "or" => {
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Or);
                ctx.emit(Instruction::I64ExtendI32S);
            }
            _ => {} // 未知の演算子は無視
        }
    }
}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}

/// 関数変換コンテキスト
struct FuncCtx {
    #[allow(dead_code)]
    name: String,
    instructions: Vec<Instruction>,
    locals_map: HashMap<String, u32>,
    param_count: u32,
    next_local: u32,
}

impl FuncCtx {
    fn new(name: String) -> Self {
        Self {
            name,
            instructions: Vec::new(),
            locals_map: HashMap::new(),
            param_count: 0,
            next_local: 0,
        }
    }

    fn emit(&mut self, instr: Instruction) {
        self.instructions.push(instr);
    }

    fn alloc_local(&mut self, name: String) -> u32 {
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
fn is_builtin_binop(name: &str) -> bool {
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
fn type_to_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Con(name) => Some(name.clone()),
        Type::Record(name, _) => Some(name.clone()),
        Type::App(name, _) => Some(name.clone()),
        _ => None,
    }
}

/// TypeExpr から型名を抽出（静的ディスパッチ用）
fn type_expr_to_name(ty: &TypeExpr) -> Option<String> {
    match ty {
        TypeExpr::Named(_, name) => Some(name.clone()),
        _ => None,
    }
}

/// TypeExpr -> IR 型（簡易変換）
fn type_expr_to_ir(ty: &lsharp_syntax::ast::TypeExpr) -> IrType {
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

#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_types::infer::Infer;

    /// ソースコードから IR モジュールを生成するヘルパー
    fn lower(source: &str) -> Module {
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        lowerer.lower_program(&program, &type_results).unwrap()
    }

    /// IR のテキストダンプをスナップショットテストで検証
    fn assert_ir(source: &str, snapshot_name: &str) {
        let module = lower(source);
        insta::assert_snapshot!(snapshot_name, module.dump());
    }

    #[test]
    fn test_lower_integer_literal() {
        assert_ir("(defn main [] 42)", "lower_integer_literal");
    }

    #[test]
    fn test_lower_bool_literal() {
        assert_ir("(defn main [] true)", "lower_bool_literal");
    }

    #[test]
    fn test_lower_arithmetic() {
        assert_ir("(defn main [] (+ (* 3 4) 5))", "lower_arithmetic");
    }

    #[test]
    fn test_lower_comparison() {
        assert_ir("(defn main [] (< 1 2))", "lower_comparison");
    }

    #[test]
    fn test_lower_if_expr() {
        assert_ir(
            "(defn main [] (if (< 1 2) 42 0))",
            "lower_if_expr",
        );
    }

    #[test]
    fn test_lower_let_binding() {
        assert_ir(
            "(defn main [] (let [x 10 y 20] (+ x y)))",
            "lower_let_binding",
        );
    }

    #[test]
    fn test_lower_nested_let() {
        assert_ir(
            "(defn main [] (let [a 5 b (+ a 3)] (* a b)))",
            "lower_nested_let",
        );
    }

    #[test]
    fn test_lower_function_call() {
        assert_ir(
            "(defn double [x] (* x 2))
             (defn main [] (double 21))",
            "lower_function_call",
        );
    }

    #[test]
    fn test_lower_recursive_function() {
        assert_ir(
            "(defn fib [n]
               (if (<= n 1)
                 n
                 (+ (fib (- n 1)) (fib (- n 2)))))
             (defn main [] (fib 10))",
            "lower_recursive_function",
        );
    }

    #[test]
    fn test_lower_print_call() {
        assert_ir(
            "(defn main [] (print 42))",
            "lower_print_call",
        );
    }

    #[test]
    fn test_lower_wildcard_let() {
        assert_ir(
            "(defn main [] (let [_ 99] 1))",
            "lower_wildcard_let",
        );
    }

    #[test]
    fn test_lower_do_block() {
        assert_ir(
            "(defn main [] (do (print 1) (print 2) 42))",
            "lower_do_block",
        );
    }

    #[test]
    fn test_lower_not_operator() {
        assert_ir(
            "(defn main [] (not true))",
            "lower_not_operator",
        );
    }

    #[test]
    fn test_lower_undefined_variable_error() {
        use lsharp_syntax::ast::*;
        use lsharp_syntax::span::Span;

        let s = Span { start: 0, end: 0 };
        let program = Program {
            decls: vec![Decl::Defn {
                span: s,
                name: "main".to_string(),
                params: vec![],
                return_ty: None,
                body: Expr::Var(s, "undefined_var".to_string()),
                where_clauses: Vec::new(),
                metadata: None,
            }],
        };
        let mut lowerer = Lower::new();
        let result = lowerer.lower_program(&program, &[]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, LowerError::UndefinedFunction { .. }),
            "expected UndefinedFunction error, got: {err}"
        );
    }

    #[test]
    fn test_lower_lambda_unsupported() {
        let program = lsharp_syntax::parse("(defn main [] (fn [x] x))").unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let result = lowerer.lower_program(&program, &type_results);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, LowerError::Unsupported { .. }),
            "expected Unsupported error, got: {err}"
        );
    }
}

#[cfg(test)]
mod private_tests {
    use super::*;
    use lsharp_types::infer::Infer;

    #[test]
    fn test_lower_private_defn() {
        // private 内の関数も正しく IR 変換される
        let program = lsharp_syntax::parse(
            "(private (defn helper [x] (+ x 1))) (defn main [] (helper 42))"
        ).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // helper と main の2つの関数が生成される
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.functions[0].name, "helper");
        assert_eq!(module.functions[1].name, "main");
    }

    #[test]
    fn test_lower_private_record() {
        // private 内のレコード型も正しく IR 変換される
        let program = lsharp_syntax::parse(
            "(private (type Point (record (: x Int) (: y Int)))) (defn main [] 42)"
        ).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // GC 型として Point が登録されている
        assert_eq!(module.gc_types.len(), 1);
        assert_eq!(module.gc_types[0].name, "Point");
    }
}

#[cfg(test)]
mod trait_impl_tests {
    use super::*;
    use lsharp_types::infer::Infer;

    #[test]
    fn test_lower_trait_impl_methods() {
        // トレイト定義 + 実装で、impl メソッドが IR 関数として生成される
        let source = r#"
            (trait (Show a)
              (defn show [x] 0))
            (impl (Show Int)
              (defn show [x] (+ x 1)))
            (defn main [] 42)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // main + Show_Int_show の 2 関数が生成される
        assert_eq!(module.functions.len(), 2);
        assert_eq!(module.functions[0].name, "main");
        assert_eq!(module.functions[1].name, "Show_Int_show");
    }

    #[test]
    fn test_lower_multiple_trait_impls() {
        // 複数型への impl
        let source = r#"
            (trait (Eq a)
              (defn eq? [x y] (== x y)))
            (impl (Eq Int)
              (defn eq? [x y] (== x y)))
            (impl (Eq Bool)
              (defn eq? [x y] (== x y)))
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // main + Eq_Int_eq? + Eq_Bool_eq? = 3
        assert_eq!(module.functions.len(), 3);

        // マングル名を確認
        let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"Eq_Int_eq?"));
        assert!(names.contains(&"Eq_Bool_eq?"));
    }

    #[test]
    fn test_trait_method_impl_resolution() {
        // trait_method_impls テーブルが正しく構築される
        let source = r#"
            (trait (Show a)
              (defn show [x] 0))
            (impl (Show Int)
              (defn show [x] (+ x 1)))
            (defn main [] 42)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let _module = lowerer.lower_program(&program, &type_results).unwrap();

        // 解決テーブルに (Show, Int, show) -> Show_Int_show が登録されている
        let key = ("Show".to_string(), "Int".to_string(), "show".to_string());
        assert_eq!(
            lowerer.trait_method_impls.get(&key),
            Some(&"Show_Int_show".to_string())
        );
    }

    #[test]
    fn test_static_dispatch_with_literal_arg() {
        // トレイトメソッド呼び出しがリテラル引数から自動解決される
        let source = r#"
            (trait (Show a)
              (defn show [x] 0))
            (impl (Show Int)
              (defn show [x] (+ x 1)))
            (defn main [] (show 42))
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // main 関数に Call 命令が含まれる（Show_Int_show への呼び出し）
        let main_func = module.functions.iter().find(|f| f.name == "main").unwrap();
        let has_call = main_func.body.iter().any(|i| matches!(i, Instruction::Call(_)));
        assert!(has_call, "main 関数にトレイトメソッド呼び出し（Call）が含まれるべき: {:?}", main_func.body);
    }

    #[test]
    fn test_static_dispatch_unique_impl() {
        // 実装が1つだけの場合、型が不明でも一意解決される
        let source = r#"
            (trait (Show a)
              (defn show [x] 0))
            (impl (Show Int)
              (defn show [x] (+ x 1)))
            (defn use-show [x] (show x))
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // use-show が正常にコンパイルされる（一意解決）
        let use_show = module.functions.iter().find(|f| f.name == "use-show").unwrap();
        let has_call = use_show.body.iter().any(|i| matches!(i, Instruction::Call(_)));
        assert!(has_call, "use-show にトレイトメソッド呼び出し（Call）が含まれるべき: {:?}", use_show.body);
    }

    #[test]
    fn test_trait_method_names_table() {
        // trait_method_names テーブルが正しく構築される
        let source = r#"
            (trait (Show a)
              (defn show [x] 0))
            (trait (Eq a)
              (defn eq? [x y] (== x y)))
            (defn main [] 0)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let _module = lowerer.lower_program(&program, &type_results).unwrap();

        assert_eq!(
            lowerer.trait_method_names.get("show"),
            Some(&vec!["Show".to_string()])
        );
        assert_eq!(
            lowerer.trait_method_names.get("eq?"),
            Some(&vec!["Eq".to_string()])
        );
    }
}

#[cfg(test)]
mod constraint_check_tests {
    use super::*;
    use lsharp_types::infer::Infer;

    #[test]
    fn test_lower_constrained_type_generates_new() {
        let source = r#"
            (type-constrained Natural Int :constraints [(>= 0)])
            (defn main [] 42)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        // main + Natural.new + Natural.valid? = 3 関数
        let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"Natural.new"), "Natural.new が生成されていない: {:?}", names);
        assert!(names.contains(&"Natural.valid?"), "Natural.valid? が生成されていない: {:?}", names);
    }

    #[test]
    fn test_constraint_check_gte_instructions() {
        let source = r#"
            (type-constrained Natural Int :constraints [(>= 0)])
            (defn main [] 42)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        let new_func = module.functions.iter().find(|f| f.name == "Natural.new").unwrap();
        // 最後の命令は LocalGet(0) (値をそのまま返す)
        assert!(matches!(
            new_func.body.last(),
            Some(Instruction::LocalGet(0))
        ));
        // Unreachable が含まれている (制約違反時のトラップ)
        assert!(
            new_func.body.iter().any(|i| matches!(i, Instruction::Unreachable)),
            "Natural.new に Unreachable が含まれていない"
        );
    }

    #[test]
    fn test_constraint_valid_returns_bool() {
        let source = r#"
            (type-constrained Natural Int :constraints [(>= 0)])
            (defn main [] 42)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        let valid_func = module.functions.iter().find(|f| f.name == "Natural.valid?").unwrap();
        // 最初の命令は I64Const(1) (true で初期化)
        assert!(matches!(valid_func.body.first(), Some(Instruction::I64Const(1))));
        // Unreachable は含まれない (valid? はトラップしない)
        assert!(
            !valid_func.body.iter().any(|i| matches!(i, Instruction::Unreachable)),
            "Natural.valid? に Unreachable が含まれてはいけない"
        );
    }

    #[test]
    fn test_constraint_range_generates_both_checks() {
        let source = r#"
            (type-constrained Port Int :constraints [(range 1 65535)])
            (defn main [] 42)
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        let new_func = module.functions.iter().find(|f| f.name == "Port.new").unwrap();
        // range は 2 つの Unreachable を生成 (下限チェック + 上限チェック)
        let unreachable_count = new_func.body.iter()
            .filter(|i| matches!(i, Instruction::Unreachable))
            .count();
        assert_eq!(unreachable_count, 2, "Range 制約は 2 つのチェックを生成する");
    }
}


#[cfg(test)]
mod record_pattern_tests {
    use super::*;
    use lsharp_types::infer::Infer;
    use crate::Instruction;

    #[test]
    fn test_record_pattern_uses_struct_get() {
        let source = r#"
            (type Point (record (: x Int) (: y Int)))
            (defn get-x [p]
              (match p
                [{Point x px y py} px]))
        "#;
        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lowerer = Lower::new();
        let module = lowerer.lower_program(&program, &type_results).unwrap();

        let get_x = module.functions.iter().find(|f| f.name == "get-x").unwrap();
        // StructGet 命令が生成されていることを確認
        let struct_gets: Vec<_> = get_x.body.iter()
            .filter(|i| matches!(i, Instruction::StructGet(_, _)))
            .collect();
        assert!(struct_gets.len() >= 1, "レコードパターンは StructGet を使用すべき: {:?}", get_x.body);
    }

    #[test]
    fn test_resolve_field_index() {
        let mut lowerer = Lower::new();
        lowerer.record_fields.insert(
            "Point".to_string(),
            vec!["x".to_string(), "y".to_string()],
        );
        assert_eq!(lowerer.resolve_field_index("Point", "x"), Some(0));
        assert_eq!(lowerer.resolve_field_index("Point", "y"), Some(1));
        assert_eq!(lowerer.resolve_field_index("Point", "z"), None);
        assert_eq!(lowerer.resolve_field_index("Unknown", "x"), None);
    }
}
