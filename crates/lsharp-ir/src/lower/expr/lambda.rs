use lsharp_syntax::ast::{Expr, Param};
use lsharp_syntax::span::Span;
use lsharp_types::{infer::ExprTypeKey, types::Type};

use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError, type_expr_to_name, type_to_name};
use crate::{Function, Instruction, IrType};

impl Lower {
    pub(super) fn lower_lambda(
        &mut self,
        ctx: &mut FuncCtx,
        lambda_span: Span,
        params: &[Param],
        body: &Expr,
    ) -> Result<(), LowerError> {
        // Lambda Lifting: Lambda 式をトップレベル関数にリフト
        let lambda_name = self.fresh_lambda_name();

        // 自由変数を検出
        let free_var_list = self.wasmgc_lambda_free_vars(params, body);

        if self.backend == LowerBackend::WasmGc && !free_var_list.is_empty() {
            return Err(LowerError::Unsupported {
                msg: "WasmGC captured closure は typed funcref/env struct への変換が未実装です"
                    .to_string(),
                span: Some(lambda_span),
            });
        }

        if self.backend == LowerBackend::WasmGc {
            let lambda_type = self
                .expr_type_results
                .get(&ExprTypeKey::new(&ctx.type_scope_key, lambda_span))
                .cloned();
            let (param_types, param_type_names, inferred_result_type) = match lambda_type {
                Some(Type::Fun(inferred_params, ret)) if inferred_params.len() == params.len() => (
                    inferred_params
                        .iter()
                        .map(|ty| self.ir_type_for_type(ty))
                        .collect::<Vec<_>>(),
                    inferred_params.iter().map(type_to_name).collect::<Vec<_>>(),
                    Some(self.ir_type_for_type(&ret)),
                ),
                _ => (
                    params
                        .iter()
                        .map(|param| {
                            param
                                .ty
                                .as_ref()
                                .map(|ty| self.type_expr_to_ir(ty))
                                .unwrap_or(IrType::I64)
                        })
                        .collect::<Vec<_>>(),
                    params
                        .iter()
                        .map(|param| param.ty.as_ref().and_then(type_expr_to_name))
                        .collect::<Vec<_>>(),
                    None,
                ),
            };

            let mut lifted_ctx =
                FuncCtx::with_type_scope(lambda_name.clone(), ctx.type_scope_key.clone());
            for (param_idx, param) in params.iter().enumerate() {
                let idx = lifted_ctx.next_local;
                lifted_ctx.locals_map.insert(param.name.clone(), idx);
                if let Some(type_name) = param_type_names.get(param_idx).cloned().flatten() {
                    lifted_ctx
                        .local_type_names
                        .insert(param.name.clone(), type_name);
                }
                lifted_ctx.param_count += 1;
                lifted_ctx.next_local += 1;
                lifted_ctx
                    .local_types
                    .push(param_types.get(param_idx).copied().unwrap_or(IrType::I64));
            }

            self.lower_expr(&mut lifted_ctx, body)?;
            let result_type = inferred_result_type
                .or_else(|| {
                    self.infer_expr_type_name_with_ctx(&lifted_ctx, body)
                        .map(|name| self.ir_type_for_type_name(&name))
                })
                .unwrap_or(IrType::I64);
            let extra_local_count = (lifted_ctx.next_local - lifted_ctx.param_count) as usize;
            let extra_locals = lifted_ctx
                .local_types
                .get(lifted_ctx.param_count as usize..)
                .filter(|types| types.len() == extra_local_count)
                .map_or_else(
                    || vec![IrType::I64; extra_local_count],
                    |types| types.to_vec(),
                );
            let lifted_func = Function {
                name: lambda_name.clone(),
                params: param_types,
                result: result_type,
                locals: extra_locals,
                body: lifted_ctx.instructions,
                is_export: false,
            };
            let func_idx = self.next_func_idx + self.lifted_functions.len() as u32;
            let local_func_idx =
                func_idx
                    .checked_sub(self.import_count)
                    .ok_or_else(|| LowerError::Unsupported {
                        msg: "WasmGC lambda の function index が runtime import 境界より前です"
                            .to_string(),
                        span: Some(lambda_span),
                    })?;
            self.lifted_func_indices.insert(lambda_name, func_idx);
            self.lifted_functions.push(lifted_func);
            ctx.emit(Instruction::RefFunc(local_func_idx));
            return Ok(());
        }

        // リフト先関数を構築:
        // 統一呼び出し規約: (元パラメータ..., closure_ptr) -> result
        // リフト関数内部で closure_ptr からキャプチャ値を読み出す
        let mut lifted_ctx =
            FuncCtx::with_type_scope(lambda_name.clone(), ctx.type_scope_key.clone());
        // 元のパラメータを登録
        for p in params {
            let idx = lifted_ctx.next_local;
            lifted_ctx.locals_map.insert(p.name.clone(), idx);
            lifted_ctx.param_count += 1;
            lifted_ctx.next_local += 1;
        }
        // closure_ptr パラメータを追加（常に最後のパラメータ）
        let closure_ptr_idx = lifted_ctx.next_local;
        lifted_ctx
            .locals_map
            .insert("__closure_ptr".to_string(), closure_ptr_idx);
        lifted_ctx.param_count += 1;
        lifted_ctx.next_local += 1;

        // 自由変数をローカルに読み出すプロローグを生成
        let mut prologue = Vec::new();
        for (i, fv) in free_var_list.iter().enumerate() {
            let fv_local = lifted_ctx.alloc_local(fv.clone());
            // closure_ptr からキャプチャ値を読み出す:
            // fv_local = i64.load(i32.wrap(closure_ptr) + 8 + i*8)
            prologue.push(Instruction::LocalGet(closure_ptr_idx));
            prologue.push(Instruction::I32WrapI64);
            prologue.push(Instruction::I64Load {
                offset: 8 + (i as u32) * 8,
            });
            prologue.push(Instruction::LocalSet(fv_local));
        }

        // Lambda の本体を変換
        self.lower_expr(&mut lifted_ctx, body)?;

        // プロローグを本体の先頭に挿入
        let mut full_body = prologue;
        full_body.extend(lifted_ctx.instructions);

        // リフト先関数のパラメータ型: (元パラメータ..., closure_ptr)
        let total_params = params.len() + 1; // +1 は closure_ptr
        let extra_locals =
            vec![IrType::I64; (lifted_ctx.next_local - lifted_ctx.param_count) as usize];

        let lifted_func = Function {
            name: lambda_name.clone(),
            params: vec![IrType::I64; total_params],
            result: IrType::I64,
            locals: extra_locals,
            body: full_body,
            is_export: false,
        };

        // リフトされた関数のインデックスを割り当て
        let func_idx = self.next_func_idx + self.lifted_functions.len() as u32;
        self.lifted_func_indices.insert(lambda_name, func_idx);
        self.lifted_functions.push(lifted_func);

        // クロージャオブジェクトをヒープに確保（自由変数の有無に関わらず）
        // レイアウト: [heap_tag=4: i32, func_idx: i32, captured_0: i64, ...]
        {
            let n_captures = free_var_list.len();
            let alloc_size = 8 + (n_captures as i64) * 8; // 最低 8 バイト (ヘッダのみ)

            // __alloc(size) でメモリ確保
            ctx.emit(Instruction::I64Const(alloc_size));
            let alloc_idx = *self.func_indices.get("__alloc").unwrap_or(&1);
            ctx.emit(Instruction::Call(alloc_idx));
            // __alloc は i64 を返す → i64 のままローカルに保存
            let addr_local = ctx.alloc_local("_closure_addr".to_string());
            ctx.emit(Instruction::LocalSet(addr_local));

            // heap_tag=4 (CLOSURE) を offset 0 に書き込む
            ctx.emit(Instruction::LocalGet(addr_local));
            ctx.emit(Instruction::I32WrapI64);
            ctx.emit(Instruction::I32Const(crate::lower::HEAP_TAG_CLOSURE));
            ctx.emit(Instruction::I32Store { offset: 0 });

            // func_idx を offset 4 に書き込む
            ctx.emit(Instruction::LocalGet(addr_local));
            ctx.emit(Instruction::I32WrapI64);
            ctx.emit(Instruction::FuncIdx(func_idx));
            ctx.emit(Instruction::I32Store { offset: 4 });

            // キャプチャ値を書き込む: mem[addr + 8 + i*8] = captured_i
            for (i, fv) in free_var_list.iter().enumerate() {
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                if let Some(&fv_local) = ctx.locals_map.get(fv) {
                    ctx.emit(Instruction::LocalGet(fv_local));
                } else {
                    // フォールバック: 0 を書き込む
                    ctx.emit(Instruction::I64Const(0));
                }
                ctx.emit(Instruction::I64Store {
                    offset: 8 + (i as u32) * 8,
                });
            }

            // タグ付きポインタを返す: addr は i64 のまま
            // 最上位ビットをセット: addr | (1 << 63)
            ctx.emit(Instruction::LocalGet(addr_local));
            ctx.emit(Instruction::I64Const(1i64 << 63));
            ctx.emit(Instruction::I64Add);
        }
        Ok(())
    }
}
