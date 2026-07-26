use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerError};
use crate::{Instruction, IrType};

impl Lower {
    pub(super) fn lower_app_vector(
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
            // vector-new: 指定 capacity で空ベクタを確保
            // レイアウト: [tag=5: i32, capacity: i32, length: i32, padding: i32, elem_0: i64, ...]
            // ヘッダ 16 バイト + 各要素 8 バイト
            "vector-new" => {
                // 引数: capacity (i64)
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                // capacity を i64 のままローカルに保存
                let cap_local = ctx.alloc_local("_vec_cap".to_string());
                ctx.emit(Instruction::LocalSet(cap_local));
                // 割り当てサイズ = 16 + capacity * 8 (i64 算術)
                ctx.emit(Instruction::I64Const(16));
                ctx.emit(Instruction::LocalGet(cap_local));
                ctx.emit(Instruction::I64Const(8));
                ctx.emit(Instruction::I64Mul);
                ctx.emit(Instruction::I64Add);
                // __alloc 呼び出し (i64 引数)
                let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                    LowerError::UndefinedFunction {
                        name: "__alloc".to_string(),
                        span: Some(expr_span),
                    }
                })?;
                ctx.emit(Instruction::Call(alloc_idx));
                // アドレスをローカルに保存 (i64)
                let addr_local = ctx.alloc_local("_vec_addr".to_string());
                ctx.emit(Instruction::LocalSet(addr_local));
                // tag=5 書き込み: mem[addr+0] = HEAP_TAG_VECTOR
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(crate::lower::HEAP_TAG_VECTOR));
                ctx.emit(Instruction::I32Store { offset: 0 });
                // capacity 書き込み: mem[addr+4] = capacity
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::LocalGet(cap_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Store { offset: 4 });
                // length=0 書き込み: mem[addr+8] = 0
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(0));
                ctx.emit(Instruction::I32Store { offset: 8 });
                // padding=0 書き込み: mem[addr+12] = 0
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I32WrapI64);
                ctx.emit(Instruction::I32Const(0));
                ctx.emit(Instruction::I32Store { offset: 12 });
                // タグ付きポインタを返す
                ctx.emit(Instruction::LocalGet(addr_local));
                ctx.emit(Instruction::I64Const(1i64 << 63));
                ctx.emit(Instruction::I64Add);
                Ok(true)
            }
            // vector-length: ベクタの現在の長さを返す
            "vector-length" => {
                if let Some(arg) = args.first() {
                    self.lower_expr(ctx, arg)?;
                }
                // タグ解除してアドレスを取得
                ctx.emit(Instruction::I32WrapI64);
                // mem[addr+8] から length を読み出す
                ctx.emit(Instruction::I32Load { offset: 8 });
                // i32 -> i64 に拡張
                ctx.emit(Instruction::I64ExtendI32U);
                Ok(true)
            }
            // vector-get: インデックス指定で要素を取得
            // vector-get [v i] -> a
            "vector-get" => {
                if args.len() >= 2 {
                    // 第1引数: ベクタ (tagged pointer) → i64 のまま保持
                    let addr_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_vget_addr",
                        "_vget_root_slot",
                    )?;
                    // 第2引数: インデックス (i64) → i32 に変換して計算
                    self.lower_expr(ctx, &args[1])?;
                    ctx.emit(Instruction::I32WrapI64);
                    // 要素のオフセット = 16 + i * 8
                    ctx.emit(Instruction::I32Const(8));
                    ctx.emit(Instruction::I32Mul);
                    ctx.emit(Instruction::I32Const(16));
                    ctx.emit(Instruction::I32Add);
                    // addr (i64 → i32) + offset
                    ctx.emit(Instruction::LocalGet(addr_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Add);
                    // i64 値を読み出す
                    ctx.emit(Instruction::I64Load { offset: 0 });
                    self.emit_root_pop_drop(ctx)?;
                }
                Ok(true)
            }
            // vector-set: インデックス指定で要素を上書き (ミューテーション)
            // vector-set [v i x] -> Vector (同じベクタを返す)
            "vector-set" => {
                if args.len() >= 3 {
                    // 第1引数: ベクタ (tagged pointer) → i64 のまま保持
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_vset_tagged",
                        "_vset_root_slot",
                    )?;
                    // 第2引数: インデックス (i64) → i32 に変換
                    self.lower_expr(ctx, &args[1])?;
                    ctx.emit(Instruction::I32WrapI64);
                    // 要素のオフセット = 16 + i * 8
                    ctx.emit(Instruction::I32Const(8));
                    ctx.emit(Instruction::I32Mul);
                    ctx.emit(Instruction::I32Const(16));
                    ctx.emit(Instruction::I32Add);
                    // addr (i64 → i32) + offset
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64);
                    ctx.emit(Instruction::I32Add);
                    // 第3引数: 新しい値 (i64)
                    self.lower_expr(ctx, &args[2])?;
                    // mem[elem_addr] = value
                    ctx.emit(Instruction::I64Store { offset: 0 });
                    self.emit_root_pop_drop(ctx)?;
                    // 同じタグ付きポインタを返す
                    ctx.emit(Instruction::LocalGet(tagged_local));
                }
                Ok(true)
            }
            // vector-push: 要素を末尾に追加 (capacity 超過時は再割り当て)
            // vector-push [v x] -> Vector
            // 注意: すべてのローカル変数は i64 型で保持
            "vector-push" => {
                if args.len() >= 2 {
                    // 第1引数: ベクタ (tagged pointer) → i64
                    let tagged_local = self.lower_expr_to_rooted_local(
                        ctx,
                        &args[0],
                        "_vpush_tagged",
                        "_vpush_tagged_root_slot",
                    )?;
                    // 第2引数: 追加する値 → i64
                    self.lower_expr(ctx, &args[1])?;
                    let val_local = ctx.alloc_local("_vpush_val".to_string());
                    ctx.emit(Instruction::LocalSet(val_local));

                    // length を読み出して i64 で保存
                    let len_local = ctx.alloc_local("_vpush_len".to_string());
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64); // untag
                    ctx.emit(Instruction::I32Load { offset: 8 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(len_local));

                    // capacity を読み出して i64 で保存
                    let cap_local = ctx.alloc_local("_vpush_cap".to_string());
                    ctx.emit(Instruction::LocalGet(tagged_local));
                    ctx.emit(Instruction::I32WrapI64); // untag
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::LocalSet(cap_local));

                    // if length >= capacity then 再割り当て else 既存バッファに追加
                    ctx.emit(Instruction::LocalGet(len_local));
                    ctx.emit(Instruction::LocalGet(cap_local));
                    ctx.emit(Instruction::I64GeS);
                    ctx.emit(Instruction::If(IrType::I64)); // 結果: 新しいタグ付きポインタ (i64)

                    // === 再割り当てブランチ ===
                    {
                        // new_cap = max(capacity * 2, 4) (i64 演算)
                        let new_cap_local = ctx.alloc_local("_vpush_newcap".to_string());
                        ctx.emit(Instruction::LocalGet(cap_local));
                        ctx.emit(Instruction::I64Const(2));
                        ctx.emit(Instruction::I64Mul);
                        let tmp_local = ctx.alloc_local("_vpush_tmp".to_string());
                        ctx.emit(Instruction::LocalSet(tmp_local));
                        ctx.emit(Instruction::LocalGet(tmp_local));
                        ctx.emit(Instruction::I64Const(4));
                        ctx.emit(Instruction::I64GtS);
                        ctx.emit(Instruction::If(IrType::I64));
                        ctx.emit(Instruction::LocalGet(tmp_local));
                        ctx.emit(Instruction::Else);
                        ctx.emit(Instruction::I64Const(4));
                        ctx.emit(Instruction::End);
                        ctx.emit(Instruction::LocalSet(new_cap_local));

                        // alloc_size = 16 + new_cap * 8 (i64)
                        ctx.emit(Instruction::I64Const(16));
                        ctx.emit(Instruction::LocalGet(new_cap_local));
                        ctx.emit(Instruction::I64Const(8));
                        ctx.emit(Instruction::I64Mul);
                        ctx.emit(Instruction::I64Add);
                        let alloc_idx = *self.func_indices.get("__alloc").ok_or_else(|| {
                            LowerError::UndefinedFunction {
                                name: "__alloc".to_string(),
                                span: Some(expr_span),
                            }
                        })?;
                        self.emit_root_push_local(ctx, val_local, "_vpush_val_root_slot")?;
                        ctx.emit(Instruction::Call(alloc_idx));
                        let new_addr_local = ctx.alloc_local("_vpush_newaddr".to_string());
                        ctx.emit(Instruction::LocalSet(new_addr_local));

                        // 新しいヘッダを書き込む
                        // tag=5
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(crate::lower::HEAP_TAG_VECTOR));
                        ctx.emit(Instruction::I32Store { offset: 0 });
                        // capacity = new_cap
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(new_cap_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Store { offset: 4 });
                        // length = old_len + 1
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I32Store { offset: 8 });
                        // padding = 0
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(0));
                        ctx.emit(Instruction::I32Store { offset: 12 });

                        // 既存要素をコピー: memory.copy(dst, src, byte_count)
                        // dst = new_addr + 16
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(16));
                        ctx.emit(Instruction::I32Add);
                        // src = old_addr + 16 (untag)
                        ctx.emit(Instruction::LocalGet(tagged_local));
                        ctx.emit(Instruction::I32WrapI64); // untag
                        ctx.emit(Instruction::I32Const(16));
                        ctx.emit(Instruction::I32Add);
                        // count = len * 8
                        ctx.emit(Instruction::LocalGet(len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Mul);
                        ctx.emit(Instruction::MemoryCopy);

                        // 新要素を書き込み: mem[new_addr + 16 + len * 8] = val
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::LocalGet(len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Mul);
                        ctx.emit(Instruction::I32Const(16));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalGet(val_local));
                        ctx.emit(Instruction::I64Store { offset: 0 });
                        self.emit_root_pop_drop(ctx)?;

                        // 新しいタグ付きポインタを返す
                        ctx.emit(Instruction::LocalGet(new_addr_local));
                        ctx.emit(Instruction::I64Const(1i64 << 63));
                        ctx.emit(Instruction::I64Add);
                    }

                    ctx.emit(Instruction::Else);

                    // === 既存バッファに追加ブランチ ===
                    {
                        // 新要素を書き込み: mem[untag(addr) + 16 + len * 8] = val
                        ctx.emit(Instruction::LocalGet(tagged_local));
                        ctx.emit(Instruction::I32WrapI64); // untag
                        ctx.emit(Instruction::LocalGet(len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(8));
                        ctx.emit(Instruction::I32Mul);
                        ctx.emit(Instruction::I32Const(16));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::LocalGet(val_local));
                        ctx.emit(Instruction::I64Store { offset: 0 });

                        // length を更新: mem[untag(addr)+8] = len + 1
                        ctx.emit(Instruction::LocalGet(tagged_local));
                        ctx.emit(Instruction::I32WrapI64); // untag
                        ctx.emit(Instruction::LocalGet(len_local));
                        ctx.emit(Instruction::I32WrapI64);
                        ctx.emit(Instruction::I32Const(1));
                        ctx.emit(Instruction::I32Add);
                        ctx.emit(Instruction::I32Store { offset: 8 });

                        // 同じタグ付きポインタを返す
                        ctx.emit(Instruction::LocalGet(tagged_local));
                    }

                    ctx.emit(Instruction::End);
                    self.emit_root_pop_drop(ctx)?;
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
