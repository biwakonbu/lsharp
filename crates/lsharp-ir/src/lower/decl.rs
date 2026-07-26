//! 宣言の lowering (関数、ADT、レコード、制約付き型等)

use lsharp_syntax::ast::*;
use lsharp_types::types::Type;

use crate::{Function, Instruction, IrType};

use super::{FuncCtx, Lower, LowerError, is_heap_like_type_name, type_expr_to_name, type_to_name};

mod self_tco;
use self_tco::{SelfTcoRootOps, apply_self_tco};

#[path = "decl/type_inference.rs"]
mod type_inference;

impl Lower {
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

    /// フィールドアクセサ関数を生成
    pub(crate) fn generate_field_accessor(
        &self,
        type_name: &str,
        field_name: &str,
        field_idx: u32,
        _field_type: &lsharp_syntax::ast::TypeExpr,
    ) -> Function {
        let accessor_name = format!("{type_name}.{field_name}");

        let mut body = Vec::new();
        if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
            body.push(Instruction::LocalGet(0));
            body.push(Instruction::StructGet(gc_type_idx, field_idx));
        } else {
            // フォールバック: 引数をそのまま返す
            body.push(Instruction::LocalGet(0));
        }

        let receiver_type = if self.backend == super::LowerBackend::WasmGc {
            self.record_type_indices
                .get(type_name)
                .copied()
                .map(IrType::Ref)
                .unwrap_or(IrType::I64)
        } else {
            IrType::I64
        };
        let field_type = self.type_expr_to_ir(_field_type);

        Function {
            name: accessor_name,
            params: vec![receiver_type],
            result: field_type,
            locals: Vec::new(),
            body,
            is_export: false,
        }
    }

    /// ADT コンストラクタ関数を生成する。
    ///
    /// ヒープレイアウト: [heap_tag=3: i32, variant_tag: i32, field_0: i64, ...]
    /// __alloc でメモリ確保 → ヘッダ書き込み → フィールド書き込み → タグ付きポインタ返却
    pub(crate) fn generate_adt_constructor(
        &self,
        variant_name: &str,
        gc_type_idx: u32,
        tag_val: i32,
        field_types: &[IrType],
        slot_types: &[IrType],
        field_offsets: &[u32],
    ) -> Function {
        let field_count = field_types.len();
        if self.backend == super::LowerBackend::WasmGc {
            let mut body = Vec::with_capacity(slot_types.len() + 2);
            body.push(Instruction::I64Const(tag_val as i64));
            for (slot_idx, slot_type) in slot_types.iter().copied().enumerate() {
                if let Some(field_idx) = field_offsets
                    .iter()
                    .position(|offset| *offset == slot_idx as u32)
                {
                    body.push(Instruction::LocalGet(field_idx as u32));
                } else {
                    match slot_type {
                        IrType::I64 | IrType::I32 => body.push(Instruction::I64Const(0)),
                        IrType::F64 => body.push(Instruction::F64Const(0.0)),
                        IrType::Ref(type_index) => {
                            body.push(Instruction::RefNull(type_index));
                        }
                        IrType::FuncRef => {
                            // FuncRef slot は WasmGC ADT payload の対象外。
                            body.push(Instruction::Unreachable);
                        }
                        IrType::TypedFuncRef(_) => {
                            // concrete funcref slot も WasmGC ADT payload の対象外。
                            body.push(Instruction::Unreachable);
                        }
                    }
                }
            }
            body.push(Instruction::StructNew(gc_type_idx));

            return Function {
                name: variant_name.to_string(),
                params: field_types.to_vec(),
                result: IrType::Ref(gc_type_idx),
                locals: Vec::new(),
                body,
                is_export: false,
            };
        }

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

        body.push(Instruction::I32Const(1));

        for constraint in constraints {
            match constraint {
                Constraint::Gte(Expr::Lit(_, Literal::Int(threshold))) => {
                    body.push(Instruction::LocalGet(0));
                    body.push(Instruction::I64Const(*threshold));
                    body.push(Instruction::I64GeS);
                    body.push(Instruction::I32And);
                }
                Constraint::Lte(Expr::Lit(_, Literal::Int(threshold))) => {
                    body.push(Instruction::LocalGet(0));
                    body.push(Instruction::I64Const(*threshold));
                    body.push(Instruction::I64LeS);
                    body.push(Instruction::I32And);
                }
                Constraint::Range(lo_expr, hi_expr) => {
                    if let (Expr::Lit(_, Literal::Int(lo)), Expr::Lit(_, Literal::Int(hi))) =
                        (lo_expr, hi_expr)
                    {
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*lo));
                        body.push(Instruction::I64GeS);
                        body.push(Instruction::I32And);
                        body.push(Instruction::LocalGet(0));
                        body.push(Instruction::I64Const(*hi));
                        body.push(Instruction::I64LeS);
                        body.push(Instruction::I32And);
                    }
                }
                _ => {}
            }
        }

        body.push(Instruction::I64ExtendI32S);

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
                        let p: Vec<IrType> = inferred_params
                            .iter()
                            .map(|ty| self.ir_type_for_type(ty))
                            .collect();
                        let param_type_names = inferred_params.iter().map(type_to_name).collect();
                        let r = self.ir_type_for_type(ret);
                        (p, r, param_type_names)
                    }
                    Type::Fun(_, ret) => {
                        let p = vec![IrType::I64; params.len()];
                        (p, self.ir_type_for_type(ret), vec![None; params.len()])
                    }
                    _ => (
                        vec![IrType::I64; params.len()],
                        self.ir_type_for_type(ty),
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
            ctx.local_types
                .push(param_types.get(param_idx).copied().unwrap_or(IrType::I64));
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
            if self.backend == super::LowerBackend::Linear
                && type_name
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
                    span: Some(body.span()),
                }
            })?;
            let root_pop_idx = *self.func_indices.get("root_pop").ok_or_else(|| {
                LowerError::UndefinedFunction {
                    name: "root_pop".to_string(),
                    span: Some(body.span()),
                }
            })?;
            let root_set_idx = *self.func_indices.get("root_set").ok_or_else(|| {
                LowerError::UndefinedFunction {
                    name: "root_set".to_string(),
                    span: Some(body.span()),
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
        let extra_local_count = (ctx.next_local - ctx.param_count) as usize;
        let extra_locals = ctx
            .local_types
            .get(ctx.param_count as usize..)
            .filter(|types| types.len() == extra_local_count)
            .map_or_else(
                || vec![IrType::I64; extra_local_count],
                |types| types.to_vec(),
            );

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
