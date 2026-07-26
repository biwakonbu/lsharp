use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::Instruction;
use crate::lower::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_app_map_lookup(
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
            // map-get [m key] -> a (未存在時は 0)
            "map-get" => {
                if args.len() >= 2 {
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_mg_tagged",
                        "_mg_root_slot",
                    )?;
                    let addr_local = ctx.alloc_local("_mg_addr".to_string());
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(addr_local));
                    let (key_local, key_is_rooted) = self.lower_map_key_to_local(
                        ctx,
                        &args[1],
                        "_mg_key_value",
                        "_mg_key_root_slot",
                        "_mg_key",
                    )?;
                    let cap_local = ctx.alloc_local("_mg_cap".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(cap_local));
                    let result_local = ctx.alloc_local("_mg_result".to_string());
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::LocalSet(result_local));
                    let i_local = ctx.alloc_local("_mg_i".to_string());
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::LocalSet(i_local));
                    ctx.emit(Instruction::BlockEmpty);
                    ctx.emit(Instruction::LoopEmpty);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::LocalGet(cap_local));
                    ctx.emit(Instruction::I64GeS);
                    ctx.emit(Instruction::BrIf(1));
                    let ea_local = ctx.alloc_local("_mg_ea".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Mul);
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalSet(ea_local));
                    let ek_local = ctx.alloc_local("_mg_ek".to_string());
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load { offset: 0 });
                    ctx.emit(Instruction::LocalSet(ek_local));
                    // found
                    ctx.emit(Instruction::LocalGet(ek_local));
                    ctx.emit(Instruction::LocalGet(key_local));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::IfEmpty);
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load { offset: 8 });
                    ctx.emit(Instruction::LocalSet(result_local));
                    ctx.emit(Instruction::Br(2));
                    ctx.emit(Instruction::End);
                    // i++ (key=0 の空スロットもスキップ — 削除後の穴を越えて探索)
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
                    ctx.emit(Instruction::LocalGet(result_local));
                }
                Ok(true)
            }
            // map-contains? [m key] -> Bool (1=存在, 0=不存在)
            "map-contains?" => {
                if args.len() >= 2 {
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_mc_tagged",
                        "_mc_root_slot",
                    )?;
                    let addr_local = ctx.alloc_local("_mc_addr".to_string());
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(addr_local));
                    let (key_local, key_is_rooted) = self.lower_map_key_to_local(
                        ctx,
                        &args[1],
                        "_mc_key_value",
                        "_mc_key_root_slot",
                        "_mc_key",
                    )?;
                    let cap_local = ctx.alloc_local("_mc_cap".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(cap_local));
                    let result_local = ctx.alloc_local("_mc_result".to_string());
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::LocalSet(result_local));
                    let i_local = ctx.alloc_local("_mc_i".to_string());
                    ctx.emit(Instruction::I64Const(0));
                    ctx.emit(Instruction::LocalSet(i_local));
                    ctx.emit(Instruction::BlockEmpty);
                    ctx.emit(Instruction::LoopEmpty);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::LocalGet(cap_local));
                    ctx.emit(Instruction::I64GeS);
                    ctx.emit(Instruction::BrIf(1));
                    let ea_local = ctx.alloc_local("_mc_ea".to_string());
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalGet(i_local));
                    ctx.emit(Instruction::I64Const(16));
                    ctx.emit(Instruction::I64Mul);
                    ctx.emit(Instruction::I64Add);
                    ctx.emit(Instruction::LocalSet(ea_local));
                    let ek_local = ctx.alloc_local("_mc_ek".to_string());
                    ctx.emit(Instruction::LocalGet(ea_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I64Load { offset: 0 });
                    ctx.emit(Instruction::LocalSet(ek_local));
                    // found
                    ctx.emit(Instruction::LocalGet(ek_local));
                    ctx.emit(Instruction::LocalGet(key_local));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::IfEmpty);
                    ctx.emit(Instruction::I64Const(1));
                    ctx.emit(Instruction::LocalSet(result_local));
                    ctx.emit(Instruction::Br(2));
                    ctx.emit(Instruction::End);
                    // i++ (全スロット走査)
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
                    ctx.emit(Instruction::LocalGet(result_local));
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
