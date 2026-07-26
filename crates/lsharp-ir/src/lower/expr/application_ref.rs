use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::Instruction;
use crate::lower::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_app_ref(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        func: &Expr,
        args: &[Expr],
    ) -> Result<bool, LowerError> {
        let Expr::Var(_, name) = func else {
            return Ok(false);
        };

        match name.as_str() {
            "ref-new" => {
                // 引数 (値) を評価
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                // 値を一時ローカルに保存
                let val_local = ctx.alloc_local("_ref_val".to_string());
                ctx.emit(Instruction::LocalSet(val_local));
                self.emit_root_push_local(ctx, val_local, "_ref_val_root_slot")?;
                // __alloc(16) でヒープ確保
                ctx.emit(Instruction::I64Const(16));
                let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__alloc".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(alloc_idx));
                // アドレスを i64 のままローカルに保存
                let addr_local = ctx.alloc_local("_ref_addr".to_string());
                ctx.emit(Instruction::LocalSet(addr_local));
                // ヘッダ書き込み: mem[addr+0] = tag=7
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(crate::lower::HEAP_TAG_REF));
                ctx.emit(Instruction::I32Store { offset: 0 });
                // ヘッダ書き込み: mem[addr+4] = size=16
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(16));
                ctx.emit(Instruction::I32Store { offset: 4 });
                // 値書き込み: mem[addr+8] = value
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::LocalGet(val_local));
                ctx.emit(Instruction::I64Store { offset: 8 });
                self.emit_root_pop_drop(ctx)?;
                // タグ付きポインタを返す (addr は既に i64)
                // 最上位ビットをセット: addr | (1 << 63)
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I64Const(1i64 << 63));
                ctx.emit(Instruction::I64Add);
                Ok(true)
            }
            // ref-get: Ref Cell から値を読み出す
            "ref-get" => {
                // 引数 (Ref ポインタ) を評価
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                // タグ解除してアドレスを取得
                crate::lower::emit_untag_pointer(&mut ctx.instructions);
                // mem[addr+8] から値を読み出す
                ctx.emit(Instruction::I64Load { offset: 8 });
                Ok(true)
            }
            // ref-set: Ref Cell に値を書き込む
            "ref-set" => {
                if args.len() >= 2 {
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_ref_set_tagged",
                        "_ref_set_root_slot",
                    )?;
                    // タグ解除してアドレスを取得
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    crate::lower::emit_untag_pointer(&mut ctx.instructions);
                    // 第2引数 (新しい値) を評価
                    self.lower_expr(ctx, &args[1])?;
                    // mem[addr+8] = new_value
                    ctx.emit(Instruction::I64Store { offset: 8 });
                    self.emit_root_pop_drop(ctx)?;
                    // Unit を返す
                    ctx.emit(Instruction::I64Const(0));
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
