use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::Instruction;
use crate::lower::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_app_map_mutation(
        &mut self,
        ctx: &mut FuncCtx,
        _expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        let Expr::Var(_, name) = func else {
            return Ok(false);
        };

        match name.as_str() {
            // map-insert [m key value] -> Map
            // 線形探索: key=0 は空スロット、16バイト/エントリ
            "map-insert" => {
                if args.len() >= 3 {
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_mi_tagged",
                        "_mi_root_slot",
                    )?;
                    let addr_local = ctx.alloc_local("_mi_addr".to_string());
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(addr_local));
                    let (key_local, key_is_rooted) = self.lower_map_key_to_local(
                        ctx,
                        &args[1],
                        "_mi_key_value",
                        "_mi_key_root_slot",
                        "_mi_key",
                    )?;
                    let value_is_rooted = self.should_root_user_call_argument(ctx, &args[2]);
                    let val_local = if value_is_rooted {
                        self.lower_expr_to_rooted_local(
                            ctx,
                            &args[2],
                            "_mi_val",
                            "_mi_val_root_slot",
                        )?
                    } else {
                        self.lower_expr(ctx, &args[2])?;
                        let val_local = ctx.alloc_local("_mi_val".to_string());
                        ctx.emit(Instruction::LocalSet(val_local));
                        val_local
                    };
                    let cap_local = ctx.alloc_local("_mi_cap".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(cap_local));
                    let i_local = ctx.alloc_local("_mi_i".to_string());
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::LocalSet(i_local));
                    ctx.emit(Instruction::BlockEmpty);
                    ctx.emit(Instruction::LoopEmpty);
                    // if i >= cap → break
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::LocalGet(cap_local));
                    ctx.emit(Instruction::I64GeS);
                    ctx.emit(Instruction::BrIf(1));
                    // entry_addr = addr + 16 + i * 16
                    let ea_local = ctx.alloc_local("_mi_ea".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Mul);
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalSet(ea_local));
                    let ek_local = ctx.alloc_local("_mi_ek".to_string());
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load { offset: 0 });
                    ctx.emit(Instruction::LocalSet(ek_local));
                    // if entry_key == 0 → 新規挿入
                    ctx.emit(Instruction::LocalGet(ek_local));
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::IfEmpty);
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::LocalGet(key_local));
                    ctx.emit(Instruction::I64Store { offset: 0 });
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::LocalGet(val_local));
                    ctx.emit(Instruction::I64Store { offset: 8 });
                    // size++
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 8 });
                    ctx.emit(Instruction::I32Const(1));
                    ctx.emit(Instruction::I32Add);
                    ctx.emit(Instruction::I32Store { offset: 8 });
                    ctx.emit(Instruction::Br(2)); // block を抜ける
                    ctx.emit(Instruction::End); // end if
                    // if entry_key == key → 上書き
                    ctx.emit(Instruction::LocalGet(ek_local));
                    ctx.emit(Instruction::LocalGet(key_local));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::IfEmpty);
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::LocalGet(val_local));
                    ctx.emit(Instruction::I64Store { offset: 8 });
                    ctx.emit(Instruction::Br(2)); // block を抜ける
                    ctx.emit(Instruction::End); // end if
                    // i++
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::I64Const(1));
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalSet(i_local));
                    ctx.emit(Instruction::Br(0)); // loop continue
                    ctx.emit(Instruction::End); // end loop
                    ctx.emit(Instruction::End); // end block
                    if value_is_rooted {
                        self.emit_root_pop_drop(ctx)?;
                    }
                    if key_is_rooted {
                        self.emit_root_pop_drop(ctx)?;
                    }
                    self.emit_root_pop_drop(ctx)?;
                    ctx.emit(Instruction::LocalGet(tagged_local));
                }
                Ok(true)
            }

            // map-remove [m key] -> Map
            "map-remove" => {
                if args.len() >= 2 {
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_mr_tagged",
                        "_mr_root_slot",
                    )?;
                    let addr_local = ctx.alloc_local("_mr_addr".to_string());
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(addr_local));
                    let (key_local, key_is_rooted) = self.lower_map_key_to_local(
                        ctx,
                        &args[1],
                        "_mr_key_value",
                        "_mr_key_root_slot",
                        "_mr_key",
                    )?;
                    let cap_local = ctx.alloc_local("_mr_cap".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(cap_local));
                    let i_local = ctx.alloc_local("_mr_i".to_string());
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::LocalSet(i_local));
                    ctx.emit(Instruction::BlockEmpty);
                    ctx.emit(Instruction::LoopEmpty);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::LocalGet(cap_local));
                    ctx.emit(Instruction::I64GeS);
                    ctx.emit(Instruction::BrIf(1));
                    let ea_local = ctx.alloc_local("_mr_ea".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Mul);
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalSet(ea_local));
                    let ek_local = ctx.alloc_local("_mr_ek".to_string());
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load { offset: 0 });
                    ctx.emit(Instruction::LocalSet(ek_local));
                    // found → key=-1 (tombstone), size--
                    ctx.emit(Instruction::LocalGet(ek_local));
                    ctx.emit(Instruction::LocalGet(key_local));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::IfEmpty);
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Const(-1i64)); // tombstone marker
                    ctx.emit(Instruction::I64Store { offset: 0 });
                    // size--
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 8 });
                    ctx.emit(Instruction::I32Const(1));
                    ctx.emit(Instruction::I32Sub);
                    ctx.emit(Instruction::I32Store { offset: 8 });
                    ctx.emit(Instruction::Br(2));
                    ctx.emit(Instruction::End);
                    // empty (key==0) → not found, break
                    ctx.emit(Instruction::LocalGet(ek_local));
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::BrIf(1));
                    // tombstone (key==-1) → skip, continue probing
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::I64Const(1));
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalSet(i_local));
                    ctx.emit(Instruction::Br(0));
                    ctx.emit(Instruction::End); // loop
                    ctx.emit(Instruction::End); // block
                    if key_is_rooted {
                        self.emit_root_pop_drop(ctx)?;
                    }
                    self.emit_root_pop_drop(ctx)?;
                    ctx.emit(Instruction::LocalGet(tagged_local));
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
