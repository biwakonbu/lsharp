//! 宣言の lowering (関数、ADT、レコード、制約付き型等)

use std::collections::HashMap;

use lsharp_syntax::ast::*;
use lsharp_types::infer::ExprTypeKey;
use lsharp_types::types::{Type, TypeVarId};

use crate::{Function, Instruction, IrType};

use super::{
    FuncCtx, Lower, LowerError, is_heap_like_type_name, type_expr_to_name, type_to_ir, type_to_name,
};

impl Lower {
    fn infer_cached_expr_type_name(&self, type_scope_key: &str, expr: &Expr) -> Option<String> {
        self.expr_type_results
            .get(&ExprTypeKey::new(type_scope_key, expr.span()))
            .and_then(type_to_name)
    }

    fn bind_type_var_name(
        &self,
        type_var_names: &mut HashMap<TypeVarId, String>,
        type_var: TypeVarId,
        actual_type_name: &str,
    ) -> bool {
        match type_var_names.get(&type_var) {
            Some(existing) => existing == actual_type_name,
            None => {
                type_var_names.insert(type_var, actual_type_name.to_string());
                true
            }
        }
    }

    fn collect_type_var_names_from_arg(
        &self,
        expected: &Type,
        actual_type_name: &str,
        type_var_names: &mut HashMap<TypeVarId, String>,
    ) -> bool {
        match expected {
            Type::Var(type_var) => {
                self.bind_type_var_name(type_var_names, *type_var, actual_type_name)
            }
            Type::Con(name) | Type::Record(name, _) | Type::App(name, _) => {
                name == actual_type_name
            }
            Type::Fun(_, _) => false,
        }
    }

    fn infer_type_name_with_type_var_names(
        &self,
        ty: &Type,
        type_var_names: &HashMap<TypeVarId, String>,
    ) -> Option<String> {
        match ty {
            Type::Var(type_var) => type_var_names.get(type_var).cloned(),
            _ => type_to_name(ty),
        }
    }

    fn infer_function_return_type_name_from_args(
        &self,
        local_type_names: &HashMap<String, String>,
        type_scope_key: &str,
        func_name: &str,
        args: &[Expr],
    ) -> Option<String> {
        let ty = self.type_results.get(func_name)?;
        let Type::Fun(params, ret) = ty else {
            return type_to_name(ty);
        };

        let mut type_var_names = HashMap::new();
        for (param_ty, arg) in params.iter().zip(args) {
            let Some(arg_type_name) =
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, arg)
            else {
                continue;
            };
            if !self.collect_type_var_names_from_arg(param_ty, &arg_type_name, &mut type_var_names)
            {
                return None;
            }
        }

        self.infer_type_name_with_type_var_names(ret, &type_var_names)
            .or_else(|| type_to_name(ret))
    }

    fn infer_uniform_type_name(
        &self,
        mut type_names: impl Iterator<Item = Option<String>>,
    ) -> Option<String> {
        let first = type_names.next().flatten()?;
        if type_names.all(|type_name| type_name.as_deref() == Some(first.as_str())) {
            Some(first)
        } else {
            None
        }
    }

    fn infer_let_body_type_name(
        &self,
        local_type_names: &HashMap<String, String>,
        type_scope_key: &str,
        bindings: &[(Pattern, Expr)],
        body: &Expr,
    ) -> Option<String> {
        let mut local_type_names = local_type_names.clone();
        for (pattern, value) in bindings {
            let value_type =
                self.infer_expr_type_name_with_locals(&local_type_names, type_scope_key, value);
            if let (Pattern::Var(_, name), Some(type_name)) = (pattern, value_type) {
                local_type_names.insert(name.clone(), type_name);
            }
        }
        self.infer_expr_type_name_with_locals(&local_type_names, type_scope_key, body)
    }

    fn infer_expr_type_name_with_locals(
        &self,
        local_type_names: &HashMap<String, String>,
        type_scope_key: &str,
        expr: &Expr,
    ) -> Option<String> {
        if let Some(type_name) = self.infer_cached_expr_type_name(type_scope_key, expr) {
            return Some(type_name);
        }
        match expr {
            // リテラルから型を推定
            Expr::Lit(_, Literal::Int(_)) => Some("Int".to_string()),
            Expr::Lit(_, Literal::Float(_)) => Some("Float".to_string()),
            Expr::Lit(_, Literal::Bool(_)) => Some("Bool".to_string()),
            Expr::Lit(_, Literal::String(_)) => Some("String".to_string()),
            Expr::Lit(_, Literal::Unit) => Some("Unit".to_string()),
            // 変数の場合、型推論結果から型名を取得
            Expr::Var(_, name) => local_type_names
                .get(name)
                .cloned()
                .or_else(|| self.type_results.get(name).and_then(type_to_name)),
            // 型注釈がある場合
            Expr::Ann(_, _, type_expr) => type_expr_to_name(type_expr),
            // レコードリテラルの場合、型名が明示的
            Expr::RecordLit(_, type_name, _) => Some(type_name.clone()),
            Expr::RecordUpdate(_, base, _) => {
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, base)
            }
            Expr::Lambda(_, _, _) => Some("Closure".to_string()),
            Expr::If(_, _, then_expr, else_expr) => self.infer_uniform_type_name(
                [
                    self.infer_expr_type_name_with_locals(
                        local_type_names,
                        type_scope_key,
                        then_expr,
                    ),
                    self.infer_expr_type_name_with_locals(
                        local_type_names,
                        type_scope_key,
                        else_expr,
                    ),
                ]
                .into_iter(),
            ),
            Expr::Match(_, _, arms) => self.infer_uniform_type_name(arms.iter().map(|arm| {
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, &arm.body)
            })),
            Expr::Do(_, exprs) => exprs.last().and_then(|expr| {
                self.infer_expr_type_name_with_locals(local_type_names, type_scope_key, expr)
            }),
            Expr::Let(_, bindings, body) => {
                self.infer_let_body_type_name(local_type_names, type_scope_key, bindings, body)
            }
            // 関数呼び出しの場合、戻り値型を推定
            Expr::App(_, func, args) => {
                if let Expr::Var(_, func_name) = func.as_ref() {
                    if let Some(type_name) = self.infer_builtin_return_type_name(func_name) {
                        return Some(type_name);
                    }
                    self.infer_function_return_type_name_from_args(
                        local_type_names,
                        type_scope_key,
                        func_name,
                        args,
                    )
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn infer_builtin_return_type_name(&self, func_name: &str) -> Option<String> {
        match func_name {
            "string-concat" | "substring" | "int-to-string" | "read-file" | "command-line-arg"
            | "read-stdin" => Some("String".to_string()),
            "vector-new" | "vector-push" | "vector-set" => Some("Vector".to_string()),
            "map-new" | "map-insert" | "map-remove" => Some("Map".to_string()),
            "ref-new" => Some("Ref".to_string()),
            _ => None,
        }
    }

    /// レコード型のフィールドインデックスを解決
    pub(crate) fn resolve_field_index(&self, type_name: &str, field_name: &str) -> Option<u32> {
        if let Some(fields) = self.record_fields.get(type_name) {
            fields
                .iter()
                .position(|f| f == field_name)
                .map(|i| i as u32)
        } else {
            None
        }
    }

    /// トレイトメソッド呼び出しを静的ディスパッチで解決（P5-6）
    ///
    /// 関数名がトレイトメソッドに該当する場合、第一引数の型情報から
    /// 具体的な実装（マングル名）を解決して関数インデックスを返す。
    pub(crate) fn resolve_trait_dispatch(
        &self,
        ctx: &FuncCtx,
        method_name: &str,
        args: &[Expr],
    ) -> Option<u32> {
        // メソッド名がトレイトメソッドとして登録されているか確認
        let trait_names = self.trait_method_names.get(method_name)?;

        // 第一引数の型名を推定
        let first_arg_type = if let Some(arg) = args.first() {
            self.infer_expr_type_name_with_ctx(ctx, arg)
        } else {
            None
        };

        if let Some(type_name) = first_arg_type {
            // (trait_name, type_name, method_name) でマングル名を検索
            for trait_name in trait_names {
                let key = (
                    trait_name.clone(),
                    type_name.clone(),
                    method_name.to_string(),
                );
                if let Some(mangled) = self.trait_method_impls.get(&key) {
                    return self.func_indices.get(mangled).copied();
                }
            }
        }

        // 型が不明な場合、実装が1つだけならそれを使う（一意解決）
        for trait_name in trait_names {
            let matching: Vec<_> = self
                .trait_method_impls
                .iter()
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
    pub(crate) fn infer_expr_type_name(&self, expr: &Expr) -> Option<String> {
        let local_type_names = HashMap::new();
        self.infer_expr_type_name_with_locals(&local_type_names, "", expr)
    }

    pub(crate) fn infer_expr_type_name_with_ctx(
        &self,
        ctx: &FuncCtx,
        expr: &Expr,
    ) -> Option<String> {
        self.infer_expr_type_name_with_locals(&ctx.local_type_names, &ctx.type_scope_key, expr)
    }

    /// フィールドアクセサ関数を生成
    pub(crate) fn generate_field_accessor(
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

    /// ADT コンストラクタ関数を生成 (リニアメモリ版)
    ///
    /// ヒープレイアウト: [heap_tag=3: i32, variant_tag: i32, field_0: i64, ...]
    /// __alloc でメモリ確保 → ヘッダ書き込み → フィールド書き込み → タグ付きポインタ返却
    pub(crate) fn generate_adt_constructor(
        &self,
        variant_name: &str,
        _gc_type_idx: u32,
        tag_val: i32,
        field_count: usize,
    ) -> Function {
        let mut body = Vec::new();
        // ローカル変数の割り当て:
        // 0..field_count: パラメータ (フィールド値)
        // field_count: _addr (i32, ヒープアドレス)
        let addr_local = field_count as u32;
        let alloc_size = 8 + (field_count as i32) * 8; // ヘッダ 8 バイト + フィールド各 8 バイト

        // __alloc(size) でメモリ確保
        body.push(Instruction::I64Const(alloc_size as i64));
        let alloc_idx = *self.func_indices.get("__alloc").unwrap_or(&1);
        body.push(Instruction::Call(alloc_idx));
        // __alloc は i64 を返す → i32 に変換してローカルに保存
        body.push(Instruction::I32WrapI64);
        body.push(Instruction::LocalSet(addr_local));

        // heap_tag=3 (ADT) を offset 0 に書き込む
        body.push(Instruction::LocalGet(addr_local));
        body.push(Instruction::I32Const(super::HEAP_TAG_ADT));
        body.push(Instruction::I32Store { offset: 0 });

        // variant_tag を offset 4 に書き込む
        body.push(Instruction::LocalGet(addr_local));
        body.push(Instruction::I32Const(tag_val));
        body.push(Instruction::I32Store { offset: 4 });

        // 各フィールドを書き込む: mem[addr + 8 + i*8] = field_i
        for i in 0..field_count {
            body.push(Instruction::LocalGet(addr_local));
            body.push(Instruction::LocalGet(i as u32));
            body.push(Instruction::I64Store {
                offset: 8 + (i as u32) * 8,
            });
        }

        // タグ付きポインタを返す: addr | (1 << 63)
        body.push(Instruction::LocalGet(addr_local));
        super::emit_tag_pointer(&mut body, addr_local);

        Function {
            name: variant_name.to_string(),
            params: vec![IrType::I64; field_count],
            result: IrType::I64,
            locals: vec![IrType::I32], // addr_local (i32)
            body,
            is_export: false,
        }
    }

    /// 制約付き型のスマートコンストラクタ (Name.new) を生成
    /// 制約を満たさない場合は unreachable (トラップ) する
    pub(crate) fn generate_constraint_check(
        &self,
        type_name: &str,
        constraints: &[Constraint],
    ) -> Function {
        let func_name = format!("{type_name}.new");
        let mut body = Vec::new();

        for constraint in constraints {
            match constraint {
                Constraint::Gte(Expr::Lit(_, Literal::Int(threshold))) => {
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
                Constraint::Lte(Expr::Lit(_, Literal::Int(threshold))) => {
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
    pub(crate) fn generate_constraint_valid(
        &self,
        type_name: &str,
        constraints: &[Constraint],
    ) -> Function {
        let func_name = format!("{type_name}.valid?");
        let mut body = Vec::new();

        body.push(Instruction::I64Const(1));

        for constraint in constraints {
            match constraint {
                Constraint::Gte(Expr::Lit(_, Literal::Int(threshold))) => {
                    body.push(Instruction::LocalGet(0));
                    body.push(Instruction::I64Const(*threshold));
                    body.push(Instruction::I64GeS);
                    body.push(Instruction::I64ExtendI32S);
                    body.push(Instruction::I32WrapI64);
                    body.push(Instruction::I32And);
                    body.push(Instruction::I64ExtendI32S);
                }
                Constraint::Lte(Expr::Lit(_, Literal::Int(threshold))) => {
                    body.push(Instruction::LocalGet(0));
                    body.push(Instruction::I64Const(*threshold));
                    body.push(Instruction::I64LeS);
                    body.push(Instruction::I64ExtendI32S);
                    body.push(Instruction::I32WrapI64);
                    body.push(Instruction::I32And);
                    body.push(Instruction::I64ExtendI32S);
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
    pub(crate) fn lower_function(
        &mut self,
        name: &str,
        params: &[Param],
        body: &Expr,
    ) -> Result<Function, LowerError> {
        // 関数型を推論結果から取得
        let (param_types, result_type, inferred_param_type_names) =
            if let Some(ty) = self.type_results.get(name) {
                match ty {
                    Type::Fun(inferred_params, ret) if inferred_params.len() == params.len() => {
                        let p: Vec<IrType> = inferred_params.iter().map(type_to_ir).collect();
                        let param_type_names = inferred_params.iter().map(type_to_name).collect();
                        let r = type_to_ir(ret);
                        (p, r, param_type_names)
                    }
                    Type::Fun(_, ret) => {
                        let p = vec![IrType::I64; params.len()];
                        (p, type_to_ir(ret), vec![None; params.len()])
                    }
                    _ => (
                        vec![IrType::I64; params.len()],
                        type_to_ir(ty),
                        vec![None; params.len()],
                    ),
                }
            } else {
                let p = vec![IrType::I64; params.len()];
                (p, IrType::I64, vec![None; params.len()])
            };

        let mut ctx = FuncCtx::with_type_scope(name.to_string(), name.to_string());

        // パラメータをローカル変数として登録
        for (param_idx, param) in params.iter().enumerate() {
            let idx = ctx.next_local;
            ctx.locals_map.insert(param.name.clone(), idx);
            if let Some(type_name) = inferred_param_type_names
                .get(param_idx)
                .cloned()
                .flatten()
                .or_else(|| param.ty.as_ref().and_then(type_expr_to_name))
            {
                ctx.local_type_names.insert(param.name.clone(), type_name);
            }
            ctx.param_count += 1;
            ctx.next_local += 1;
        }

        // 本体を変換
        self.lower_expr(&mut ctx, body)?;

        let mut self_tco_root_slots = Vec::new();
        for (param_idx, param) in params.iter().enumerate() {
            let type_name = inferred_param_type_names
                .get(param_idx)
                .cloned()
                .flatten()
                .or_else(|| param.ty.as_ref().and_then(type_expr_to_name));
            if type_name
                .as_deref()
                .map(is_heap_like_type_name)
                .unwrap_or(false)
            {
                let slot_local = ctx.alloc_local(format!("_self_tco_param{param_idx}_root_slot"));
                self_tco_root_slots.push((param_idx as u32, slot_local));
            }
        }

        // 自己末尾呼び出し最適化 (Self TCO) を適用
        let body_instructions = if let Some(&self_idx) = self.func_indices.get(name) {
            let root_push_idx = *self.func_indices.get("root_push").ok_or_else(|| {
                LowerError::UndefinedFunction {
                    name: "root_push".to_string(),
                }
            })?;
            let root_pop_idx = *self.func_indices.get("root_pop").ok_or_else(|| {
                LowerError::UndefinedFunction {
                    name: "root_pop".to_string(),
                }
            })?;
            let root_set_idx = *self.func_indices.get("root_set").ok_or_else(|| {
                LowerError::UndefinedFunction {
                    name: "root_set".to_string(),
                }
            })?;
            let root_ops = SelfTcoRootOps {
                rooted_params: &self_tco_root_slots,
                root_push_idx,
                root_pop_idx,
                root_set_idx,
            };
            apply_self_tco(
                ctx.instructions,
                self_idx,
                ctx.param_count,
                result_type,
                &root_ops,
            )
        } else {
            ctx.instructions
        };

        // ローカル変数（パラメータ以外）
        let extra_locals = vec![IrType::I64; (ctx.next_local - ctx.param_count) as usize];

        Ok(Function {
            name: name.to_string(),
            params: param_types,
            result: result_type,
            locals: extra_locals,
            body: body_instructions,
            is_export: name == "main",
        })
    }
}

/// 自己末尾呼び出し最適化 (Self TCO) を適用する
///
/// 関数本体の命令列を解析し、自己再帰末尾呼び出しをループ+ジャンプに変換する。
///
/// 変換例 (`append-byte-vector` の場合):
/// ```text
/// ;; 変換前 (再帰)
/// if (i64)
///   local.get 0   ;; base case
/// else
///   ... (新しい引数を計算)
///   call self
/// end
///
/// ;; 変換後 (ループ)
/// loop (i64)
///   if (i64)
///     local.get 0
///   else
///     ...
///     local.set 3, local.set 2, local.set 1, local.set 0
///     br 1   ;; loop 再起動
///   end
/// end (loop)
/// ```
///
/// 検出条件: `Call(self_idx)` の後続命令が全て `End` のみである場合を末尾呼び出しとみなす。
/// 既存の Loop/Block 命令が含まれる関数には適用しない。
struct SelfTcoRootOps<'a> {
    rooted_params: &'a [(u32, u32)],
    root_push_idx: u32,
    root_pop_idx: u32,
    root_set_idx: u32,
}

fn apply_self_tco(
    instructions: Vec<Instruction>,
    self_idx: u32,
    param_count: u32,
    result_type: IrType,
    root_ops: &SelfTcoRootOps<'_>,
) -> Vec<Instruction> {
    // 既存のループ/ブロック命令がある場合はスキップ (安全のため)
    let has_loop_or_block = instructions.iter().any(|i| {
        matches!(
            i,
            Instruction::Loop(_)
                | Instruction::LoopEmpty
                | Instruction::Block(_)
                | Instruction::BlockEmpty
        )
    });
    if has_loop_or_block {
        return instructions;
    }

    // 自己末尾呼び出し候補を収集: position → depth at call site
    let tail_calls = find_simple_self_tail_calls(&instructions, self_idx);

    if tail_calls.is_empty() {
        return instructions;
    }

    // 変換: Loop(result_type) でラップし、各 Call(self) を LocalSets + Br に置換
    let mut result = Vec::with_capacity(
        instructions.len()
            + 2
            + root_ops.rooted_params.len() * 5
            + tail_calls.len() * (param_count as usize + 1 + root_ops.rooted_params.len() * 4),
    );
    for (param_idx, slot_local) in root_ops.rooted_params {
        result.push(Instruction::LocalGet(*param_idx));
        result.push(Instruction::Call(root_ops.root_push_idx));
        result.push(Instruction::LocalSet(*slot_local));
    }
    result.push(Instruction::Loop(result_type));

    for (i, instr) in instructions.into_iter().enumerate() {
        if let Some(&depth) = tail_calls.get(&i) {
            // Call(self) を引数ローカルへの LocalSet + Br に置き換える
            // スタック上の引数は LIFO のため、最後の引数から逆順に pop する
            for p in (0..param_count).rev() {
                result.push(Instruction::LocalSet(p));
            }
            for (param_idx, slot_local) in root_ops.rooted_params {
                result.push(Instruction::LocalGet(*slot_local));
                result.push(Instruction::LocalGet(*param_idx));
                result.push(Instruction::Call(root_ops.root_set_idx));
                result.push(Instruction::Drop);
            }
            result.push(Instruction::Br(depth));
            // Call 命令自体は出力しない (replace)
        } else {
            result.push(instr);
        }
    }

    result.push(Instruction::End); // Loop を閉じる
    for _ in root_ops.rooted_params {
        result.push(Instruction::Call(root_ops.root_pop_idx));
        result.push(Instruction::Drop);
    }
    result
}

/// 単純な自己末尾呼び出しを検出する
///
/// `Call(self_idx)` の後続命令が全て `End` のみの場合を末尾呼び出しとみなす。
/// 戻り値: position → depth (呼び出し時点の if/else ネスト深度) のマップ
fn find_simple_self_tail_calls(instructions: &[Instruction], self_idx: u32) -> HashMap<usize, u32> {
    let mut result = HashMap::new();
    let mut depth = 0i32;

    for (pos, instr) in instructions.iter().enumerate() {
        match instr {
            Instruction::If(_) | Instruction::IfEmpty => depth += 1,
            Instruction::Else => {} // depth は変化しない
            Instruction::End => depth -= 1,
            Instruction::Call(idx) if *idx == self_idx => {
                let d = depth;
                // 後続命令が全て End かつ数が depth と一致すれば末尾呼び出し
                let remaining = &instructions[pos + 1..];
                if remaining.len() == d as usize
                    && remaining.iter().all(|i| matches!(i, Instruction::End))
                {
                    result.insert(pos, d as u32);
                }
            }
            _ => {}
        }
    }
    result
}
