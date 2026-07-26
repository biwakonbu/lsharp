use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use super::helpers::validate_wasmgc_substring_static_range;
use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError};
use crate::{Instruction, IrType};

impl Lower {
    pub(super) fn lower_app_string_heap(
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
            // string-char-at: ヒープ上 String オブジェクトのバイト値を返す
            // String オブジェクト: [tag:i32=1][len:i32][bytes:u8*]
            "string-char-at" => {
                if args.len() >= 2 {
                    if self.backend == LowerBackend::WasmGc {
                        let type_index = self.string_array_type_index.ok_or_else(|| {
                            LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        self.lower_expr(ctx, &args[0])?;
                        self.lower_expr(ctx, &args[1])?;
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::ArrayGet(type_index));
                        ctx.emit(Instruction::I64ExtendI32U);
                        return Ok(true);
                    }

                    let str_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_char_at_str",
                        "_char_at_root_slot",
                    )?;
                    self.lower_expr(ctx, &args[1])?; // i (index)
                    // i を一時ローカルに保存 (i64 のまま)
                    let idx_local = ctx.alloc_local("_char_at_idx".to_string());
                    ctx.emit(Instruction::LocalSet(idx_local));
                    // bytes_addr = s + 8 (tag=4bytes, len=4bytes をスキップ)
                    ctx.emit(Instruction::LocalGet(str_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(8));
                    ctx.emit(Instruction::I32Add);
                    // addr = bytes_addr + i
                    ctx.emit(Instruction::LocalGet(idx_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Add);
                    // バイト値を読み出し
                    ctx.emit(Instruction::I32Load8U { offset: 0 });
                    // i32 -> i64 に拡張
                    ctx.emit(Instruction::I64ExtendI32U);
                    self.emit_root_pop_drop(ctx)?;
                }
                Ok(true)
            }
            // substring: ヒープ上 String オブジェクトの [start, end) を部分文字列として返す
            // 新しい String オブジェクト [tag=1, len, bytes] をヒープに確保
            "substring" => {
                if args.len() >= 3 {
                    if self.backend == LowerBackend::WasmGc {
                        validate_wasmgc_substring_static_range(&args[..3], expr_span)?;
                        let type_index = self.string_array_type_index.ok_or_else(|| {
                            LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        let str_local = ctx.alloc_local_typed(
                            "_str_substring_source".to_string(),
                            IrType::Ref(type_index),
                        );
                        let start_local =
                            ctx.alloc_local_typed("_str_substring_start".to_string(), IrType::I32);
                        let start_value_local = ctx.alloc_local_typed(
                            "_str_substring_start_value".to_string(),
                            IrType::I64,
                        );
                        let end_local =
                            ctx.alloc_local_typed("_str_substring_end".to_string(), IrType::I32);
                        let end_value_local = ctx
                            .alloc_local_typed("_str_substring_end_value".to_string(), IrType::I64);
                        let length_local =
                            ctx.alloc_local_typed("_str_substring_length".to_string(), IrType::I32);
                        let result_local = ctx.alloc_local_typed(
                            "_str_substring_result".to_string(),
                            IrType::Ref(type_index),
                        );
                        let index_local =
                            ctx.alloc_local_typed("_str_substring_index".to_string(), IrType::I32);

                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::LocalSet(str_local));
                        self.lower_expr(ctx, &args[1])?;
                        ctx.emit(Instruction::LocalSet(start_value_local));
                        self.lower_expr(ctx, &args[2])?;
                        ctx.emit(Instruction::LocalSet(end_value_local));

                        self.emit_wasmgc_substring_range_guard(
                            ctx,
                            str_local,
                            start_value_local,
                            end_value_local,
                            type_index,
                        );

                        ctx.emit(Instruction::LocalGet(start_value_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalSet(start_local));
                        ctx.emit(Instruction::LocalGet(end_value_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalSet(end_local));

                        ctx.emit(Instruction::LocalGet(end_local));
                        ctx.emit(Instruction::LocalGet(start_local));
                        ctx.emit(Instruction::I32Sub);
                        ctx.emit(Instruction::LocalSet(length_local));
                        ctx.emit(Instruction::LocalGet(length_local));
                        ctx.emit(Instruction::ArrayNewDefault(type_index));
                        ctx.emit(Instruction::LocalSet(result_local));

                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::BlockEmpty);
                        ctx.emit(Instruction::LoopEmpty);
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::LocalGet(length_local));
                        ctx.emit(Instruction::I32GeU);
                        ctx.emit(Instruction::BrIf(1));
                        ctx.emit(Instruction::LocalGet(result_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::LocalGet(str_local));
                        ctx.emit(Instruction::LocalGet(start_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::ArrayGet(type_index));
                        ctx.emit(Instruction::ArraySet(type_index));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::Br(0));
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::LocalGet(result_local));
                        return Ok(true);
                    }
                    let str_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_substr_str",
                        "_substr_root_slot",
                    )?;
                    self.lower_expr(ctx, &args[1])?; // start: i64
                    let start_local = ctx.alloc_local("_substr_start".to_string());
                    ctx.emit(Instruction::LocalSet(start_local));
                    self.lower_expr(ctx, &args[2])?; // end: i64
                    let end_local = ctx.alloc_local("_substr_end".to_string());
                    ctx.emit(Instruction::LocalSet(end_local));
                    // new_len = end - start (i64)
                    let new_len_local = ctx.alloc_local("_substr_len".to_string());
                    ctx.emit(Instruction::LocalGet(end_local));
                    ctx.emit(Instruction::LocalGet(start_local));
                    ctx.emit(Instruction::I64Sub);
                    ctx.emit(Instruction::LocalSet(new_len_local));
                    // new_obj = __alloc(8 + new_len) -- tag(4) + len(4) + bytes
                    let obj_local = ctx.alloc_local("_substr_obj".to_string());
                    ctx.emit(Instruction::LocalGet(new_len_local));
                    ctx.emit(Instruction::I64Const(8));
                    ctx.emit(Instruction::I64Add);
                    let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                        LowerError::UndefinedFunction {
                            name: "__alloc".to_string(),
                            span: Some(expr_span),
                        }
                    })?;
                    ctx.emit(Instruction::Call(alloc_idx));
                    ctx.emit(Instruction::LocalSet(obj_local));
                    // tag = String を書き込み (obj + 0)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(crate::lower::HEAP_TAG_STRING));
                    ctx.emit(Instruction::I32Store { offset: 0 });
                    // len を書き込み (obj + 4)
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::LocalGet(new_len_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Store { offset: 4 });
                    // memory.copy(obj + 8, src + 8 + start, new_len)
                    // dst: obj + 8
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(8));
                    ctx.emit(Instruction::I32Add);
                    // src: s + 8 + start
                    ctx.emit(Instruction::LocalGet(str_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Const(8));
                    ctx.emit(Instruction::I32Add);
                    ctx.emit(Instruction::LocalGet(start_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Add);
                    // len: new_len (i32)
                    ctx.emit(Instruction::LocalGet(new_len_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::MemoryCopy);
                    self.emit_root_pop_drop(ctx)?;
                    // タグ付き String handle を返す
                    ctx.emit(Instruction::LocalGet(obj_local));
                    ctx.emit(Instruction::I64Const(1i64 << 63));
                    ctx.emit(Instruction::I64Add);
                }
                Ok(true)
            }
            // string-concat: 2 つの文字列を結合
            "string-concat" => {
                if args.len() >= 2 {
                    if self.backend == LowerBackend::WasmGc {
                        let type_index = self.string_array_type_index.ok_or_else(|| {
                            LowerError::Unsupported {
                                msg: "WasmGC String の GC array type が登録されていません"
                                    .to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        let lhs_local = ctx.alloc_local_typed(
                            "_str_concat_lhs".to_string(),
                            IrType::Ref(type_index),
                        );
                        let rhs_local = ctx.alloc_local_typed(
                            "_str_concat_rhs".to_string(),
                            IrType::Ref(type_index),
                        );
                        let lhs_len_local =
                            ctx.alloc_local_typed("_str_concat_lhs_len".to_string(), IrType::I32);
                        let rhs_len_local =
                            ctx.alloc_local_typed("_str_concat_rhs_len".to_string(), IrType::I32);
                        let total_len_local =
                            ctx.alloc_local_typed("_str_concat_total_len".to_string(), IrType::I32);
                        let result_local = ctx.alloc_local_typed(
                            "_str_concat_result".to_string(),
                            IrType::Ref(type_index),
                        );
                        let index_local =
                            ctx.alloc_local_typed("_str_concat_index".to_string(), IrType::I32);

                        self.lower_expr(ctx, &args[0])?;
                        ctx.emit(Instruction::LocalSet(lhs_local));
                        self.lower_expr(ctx, &args[1])?;
                        ctx.emit(Instruction::LocalSet(rhs_local));

                        ctx.emit(Instruction::LocalGet(lhs_local));
                        ctx.emit(Instruction::ArrayLen(type_index));
                        ctx.emit(Instruction::LocalSet(lhs_len_local));
                        ctx.emit(Instruction::LocalGet(rhs_local));
                        ctx.emit(Instruction::ArrayLen(type_index));
                        ctx.emit(Instruction::LocalSet(rhs_len_local));
                        ctx.emit(Instruction::LocalGet(lhs_len_local));
                        ctx.emit(Instruction::LocalGet(rhs_len_local));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalSet(total_len_local));
                        ctx.emit(Instruction::LocalGet(total_len_local));
                        ctx.emit(Instruction::ArrayNewDefault(type_index));
                        ctx.emit(Instruction::LocalSet(result_local));

                        // lhs の bytes を新しい array の先頭へコピーする。
                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::BlockEmpty);
                        ctx.emit(Instruction::LoopEmpty);
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::LocalGet(lhs_len_local));
                        ctx.emit(Instruction::I32GeU);
                        ctx.emit(Instruction::BrIf(1));
                        ctx.emit(Instruction::LocalGet(result_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::LocalGet(lhs_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::ArrayGet(type_index));
                        ctx.emit(Instruction::ArraySet(type_index));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::Br(0));
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::End);

                        // rhs の bytes は lhs の長さの後ろへコピーする。
                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::BlockEmpty);
                        ctx.emit(Instruction::LoopEmpty);
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::LocalGet(rhs_len_local));
                        ctx.emit(Instruction::I32GeU);
                        ctx.emit(Instruction::BrIf(1));
                        ctx.emit(Instruction::LocalGet(result_local));
                        ctx.emit(Instruction::LocalGet(lhs_len_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalGet(rhs_local));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::ArrayGet(type_index));
                        ctx.emit(Instruction::ArraySet(type_index));
                        ctx.emit(Instruction::LocalGet(index_local));
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalSet(index_local));
                        ctx.emit(Instruction::Br(0));
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::LocalGet(result_local));
                        return Ok(true);
                    }
                    let lhs_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_strcat_lhs",
                        "_strcat_lhs_root_slot",
                    )?;
                    self.lower_expr(ctx, &args[1])?;
                    let rhs_local = ctx.alloc_local("_strcat_rhs".to_string());
                    ctx.emit(Instruction::LocalSet(rhs_local));
                    self.emit_root_push_local(ctx, rhs_local, "_strcat_rhs_root_slot")?;
                    let idx = *self.func_indices.get("__string_concat").ok_or_else(|| {
                        LowerError::UndefinedFunction {
                            name: "__string_concat".to_string(),
                            span: Some(expr_span),
                        }
                    })?;
                    let result_local = ctx.alloc_local("_strcat_result".to_string());
                    ctx.emit(Instruction::LocalGet(lhs_local));
                    ctx.emit(Instruction::LocalGet(rhs_local));
                    ctx.emit(Instruction::Call(idx));
                    ctx.emit(Instruction::LocalSet(result_local));
                    self.emit_root_pop_drop(ctx)?;
                    self.emit_root_pop_drop(ctx)?;
                    ctx.emit(Instruction::LocalGet(result_local));
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
