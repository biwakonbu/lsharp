use lsharp_syntax::{ast::Literal, span::Span};

use crate::{
    Instruction,
    lower::{HEAP_TAG_STRING, LowerBackend},
};

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    pub(super) fn lower_lit(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        lit: &Literal,
    ) -> Result<(), LowerError> {
        match lit {
            Literal::Int(n) => ctx.emit(Instruction::I64Const(*n)),
            Literal::Float(n) => ctx.emit(Instruction::F64Const(*n)),
            Literal::Bool(b) => ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 })),
            Literal::String(s) => {
                if self.backend == LowerBackend::WasmGc {
                    let type_index =
                        self.string_array_type_index
                            .ok_or_else(|| LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(expr_span),
                            })?;
                    for byte in s.as_bytes() {
                        ctx.emit(Instruction::I32Const(i32::from(*byte)));
                    }
                    ctx.emit(Instruction::ArrayNewFixed(
                        type_index,
                        s.len().try_into().map_err(|_| LowerError::Unsupported {
                            msg: "WasmGC String literal が array.new_fixed の長さを超えています"
                                .to_string(),
                            span: Some(expr_span),
                        })?,
                    ));
                    return Ok(());
                }

                // 文字列リテラル: データセクションにバイト列を格納し、
                // ランタイムでヒープ上に String オブジェクト [tag=1, len, bytes] を確保
                let bytes = s.as_bytes().to_vec();
                let len = bytes.len() as u32;
                let data_offset = self.string_offset;
                let label = format!("$str{}", self.string_data.len());
                self.string_data.push((label, bytes));
                self.string_offset += len;

                let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__alloc".to_string(),
                        span: Some(expr_span),
                    }
                })?;

                // __alloc(8 + len) でヒープ領域を確保
                ctx.emit(Instruction::I64Const((8 + len) as i64));
                ctx.emit(Instruction::Call(alloc_idx));
                // 戻り値 = ヒープオブジェクトのアドレス (i64)
                let obj_local = ctx.alloc_local("_str_obj".to_string());
                ctx.emit(Instruction::LocalSet(obj_local));

                // tag = String を書き込み (obj + 0)
                ctx.emit(Instruction::LocalGet(obj_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(HEAP_TAG_STRING));
                ctx.emit(Instruction::I32Store { offset: 0 });

                // len を書き込み (obj + 4)
                ctx.emit(Instruction::LocalGet(obj_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(len as i32));
                ctx.emit(Instruction::I32Store { offset: 4 });

                if len > 0 {
                    // memory.copy(obj + 8, data_offset, len)
                    // dst: obj + 8
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(8));
                    ctx.emit(Instruction::I32Add);
                    // src: data_offset (データセクション上のアドレス)
                    ctx.emit(Instruction::I32Const(data_offset as i32));
                    // len
                    ctx.emit(Instruction::I32Const(len as i32));
                    ctx.emit(Instruction::MemoryCopy);
                }

                // タグ付き String handle をスタックに積む
                ctx.emit(Instruction::LocalGet(obj_local));
                ctx.emit(Instruction::I64Const(1i64 << 63));
                ctx.emit(Instruction::I64Add);
            }
            Literal::Unit => ctx.emit(Instruction::I64Const(0)),
        }
        Ok(())
    }
}
