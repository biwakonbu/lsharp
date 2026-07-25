use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError};
use crate::{Instruction, IrType};

impl Lower {
    pub(super) fn lower_app_calls(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        match func {
            Expr::Var(_, name) if name == "root_push" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                let idx = *self.func_indices.get("root_push").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "root_push".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                Ok(true)
            }
            Expr::Var(_, name) if name == "root_pop" => {
                let idx = *self.func_indices.get("root_pop").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "root_pop".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                Ok(true)
            }
            Expr::Var(_, name) if name == "root_set" => {
                if let Some(slot) = args.first() {
                    self.lower_expr(ctx, slot)?;
                }
                if let Some(value) = args.get(1) {
                    self.lower_expr(ctx, value)?;
                }
                let idx = *self.func_indices.get("root_set").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "root_set".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(idx));
                Ok(true)
            }

            // write-file-bytes: Vector の各要素の下位 8 bit を raw bytes として書き込む。
            // 専用 IR 命令にして既存 import index を増やさず、helper 内の割り当て中も
            // path/vector を root stack で保持する。
            Expr::Var(_, name) if name == "write-file-bytes" => {
                if args.len() >= 2 {
                    let path_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_write_bytes_path",
                        "_write_bytes_path_root_slot",
                    )?;
                    let bytes_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[1],
                        "_write_bytes_value",
                        "_write_bytes_value_root_slot",
                    )?;
                    ctx.emit(Instruction::LocalGet(path_local));
                    ctx.emit(Instruction::LocalGet(bytes_local));
                    ctx.emit(Instruction::WriteFileBytes);
                    self.emit_root_pop_drop(ctx)?;
                    self.emit_root_pop_drop(ctx)?;
                }
                Ok(true)
            }

            // TypeName.field アクセサ呼び出し
            Expr::Var(_, name)
                if name.contains('.') && name.starts_with(|c: char| c.is_ascii_uppercase()) =>
            {
                // 引数（レコード）を評価
                for arg in args {
                    self.lower_expr(ctx, arg)?;
                }
                if let Some(&idx) = self.func_indices.get(name.as_str()) {
                    ctx.emit(Instruction::Call(idx));
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                        span: Some(expr_span),
                    });
                }
                Ok(true)
            }
            // ユーザー定義関数呼び出し（トレイト静的ディスパッチ対応）
            Expr::Var(_, name) => {
                if let Some(&idx) = self.func_indices.get(name.as_str()) {
                    // 既知の関数: 引数を評価して直接呼び出し
                    let is_self_recursive_call = name == &ctx.function_name;
                    let mut rooted_arg_count = 0usize;
                    for (arg_idx, arg) in args.iter().enumerate() {
                        if !is_self_recursive_call && self.should_root_user_call_argument(ctx, arg)
                        {
                            let value_local = self.lower_expr_to_rooted_local(
                                ctx,
                                arg,
                                &format!("_call_arg{arg_idx}_value"),
                                &format!("_call_arg{arg_idx}_root_slot"),
                            )?;
                            ctx.emit(Instruction::LocalGet(value_local));
                            rooted_arg_count += 1;
                        } else {
                            self.lower_expr(ctx, arg)?;
                        }
                    }
                    ctx.emit(Instruction::Call(idx));
                    for _ in 0..rooted_arg_count {
                        self.emit_root_pop_drop(ctx)?;
                    }
                } else if let Some(&idx) = self.lifted_func_indices.get(name.as_str()) {
                    // Lambda Lifting で生成された関数の呼び出し
                    for arg in args {
                        self.lower_expr(ctx, arg)?;
                    }
                    ctx.emit(Instruction::Call(idx));
                } else if self.backend == LowerBackend::WasmGc
                    && let Some(&local_idx) = ctx.locals_map.get(name)
                    && let Some(IrType::TypedFuncRef(call_ref_type_index)) =
                        ctx.local_types.get(local_idx as usize).copied()
                {
                    // WasmGC の local funcref は linear-memory closure pointer として
                    // 扱わず、concrete typed funcref local を引数の後ろに積んで
                    // typed `call_ref` する。
                    for arg in args {
                        self.lower_expr(ctx, arg)?;
                    }
                    ctx.emit(Instruction::LocalGet(local_idx));
                    ctx.emit(Instruction::CallRef(call_ref_type_index));
                } else if self.backend == LowerBackend::WasmGc
                    && let Some(&local_idx) = ctx.locals_map.get(name)
                    && let Some(IrType::Ref(env_type_index)) =
                        ctx.local_types.get(local_idx as usize).copied()
                    && let Some(call_ref_type_index) = self.wasmgc_env_call_ref_type(env_type_index)
                {
                    // captured env local は関数 ref と env ref を同じ struct から取り出し、
                    // lifted function の末尾 env parameter として call_ref する。
                    for arg in args {
                        self.lower_expr(ctx, arg)?;
                    }
                    ctx.emit(Instruction::LocalGet(local_idx));
                    ctx.emit(Instruction::LocalGet(local_idx));
                    ctx.emit(Instruction::StructGet(env_type_index, 0));
                    ctx.emit(Instruction::CallRef(call_ref_type_index));
                } else if let Some(idx) = self.resolve_trait_dispatch(ctx, name, args) {
                    // P5-6: トレイトメソッドの静的ディスパッチ自動解決
                    let mut rooted_arg_count = 0usize;
                    for (arg_idx, arg) in args.iter().enumerate() {
                        if self.should_root_user_call_argument(ctx, arg) {
                            let value_local = self.lower_expr_to_rooted_local(
                                ctx,
                                arg,
                                &format!("_trait_arg{arg_idx}_value"),
                                &format!("_trait_arg{arg_idx}_root_slot"),
                            )?;
                            ctx.emit(Instruction::LocalGet(value_local));
                            rooted_arg_count += 1;
                        } else {
                            self.lower_expr(ctx, arg)?;
                        }
                    }
                    ctx.emit(Instruction::Call(idx));
                    for _ in 0..rooted_arg_count {
                        self.emit_root_pop_drop(ctx)?;
                    }
                } else if let Some(&local_idx) = ctx.locals_map.get(name) {
                    // ローカル変数に格納されたクロージャの間接呼び出し
                    // 統一呼び出し規約: (元引数..., closure_ptr) -> result
                    // call_indirect のスタック: [arg0, ..., argN, closure_ptr, table_idx]

                    let mut rooted_arg_count = 0usize;
                    self.emit_root_push_local(ctx, local_idx, "_closure_call_root_slot")?;
                    rooted_arg_count += 1;

                    // 1. 元引数を評価してスタックに積む
                    for (arg_idx, arg) in args.iter().enumerate() {
                        if self.should_root_user_call_argument(ctx, arg) {
                            let value_local = self.lower_expr_to_rooted_local(
                                ctx,
                                arg,
                                &format!("_closure_call_arg{arg_idx}_value"),
                                &format!("_closure_call_arg{arg_idx}_root_slot"),
                            )?;
                            ctx.emit(Instruction::LocalGet(value_local));
                            rooted_arg_count += 1;
                        } else {
                            self.lower_expr(ctx, arg)?;
                        }
                    }
                    // 2. クロージャポインタをスタックに積む（リフト関数の最後のパラメータ）
                    ctx.emit(Instruction::LocalGet(local_idx));
                    // 3. テーブルインデックス (func_idx) を取得してスタックに積む
                    //    クロージャポインタからタグ解除して func_idx を読み出す
                    ctx.emit(Instruction::LocalGet(local_idx));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    // 4. call_indirect: 型は (i64 * (args.len() + 1)) -> i64
                    let call_type_id = args.len() as u32 + 1; // 元引数 + closure_ptr
                    ctx.emit(Instruction::CallIndirect(call_type_id));
                    for _ in 0..rooted_arg_count {
                        self.emit_root_pop_drop(ctx)?;
                    }
                } else {
                    return Err(LowerError::UndefinedFunction {
                        name: name.clone(),
                        span: Some(expr_span),
                    });
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
