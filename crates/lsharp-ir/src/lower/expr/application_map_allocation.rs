use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::Instruction;
use crate::lower::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_app_map_allocation(
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
            "map-new" => {
                let default_cap: i32 = 4096;
                let alloc_size: i64 = 16 + (default_cap as i64) * 16; // 65552
                ctx.emit(Instruction::I64Const(alloc_size));
                let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__alloc".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(alloc_idx));
                let addr_local = ctx.alloc_local("_map_addr".to_string());
                ctx.emit(Instruction::LocalSet(addr_local));
                // tag=6
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(crate::lower::HEAP_TAG_HASHMAP));
                ctx.emit(Instruction::I32Store { offset: 0 });
                // capacity
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(default_cap));
                ctx.emit(Instruction::I32Store { offset: 4 });
                // size=0
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(0));
                ctx.emit(Instruction::I32Store { offset: 8 });
                // エントリ領域をゼロ初期化 (key=0 は空スロット)
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(16)); // ヘッダスキップ
                ctx.emit(Instruction::I32Add);
                ctx.emit(Instruction::I32Const(0)); // fill value = 0
                ctx.emit(Instruction::I32Const(default_cap * 16)); // 65536 bytes
                ctx.emit(Instruction::MemoryFill);
                // タグ付きポインタを返す
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I64Const(1i64 << 63));
                ctx.emit(Instruction::I64Add);
                Ok(true)
            }

            // map-size: ハッシュマップのエントリ数を返す
            "map-size" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                // タグ解除してアドレスを取得
                ctx.emit(Instruction::I32WrapI64);
                // mem[addr+8] から size を読み出す
                ctx.emit(Instruction::I32Load { offset: 8 });
                // i32 -> i64 に拡張
                ctx.emit(Instruction::I64ExtendI32U);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
