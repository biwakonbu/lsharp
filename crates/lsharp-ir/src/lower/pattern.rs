//! パターンマッチの lowering

use lsharp_syntax::ast::*;

use crate::{Instruction, IrType};

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    /// match の腕を if-else チェインに変換
    pub(crate) fn lower_match_arms(
        &self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        arms: &[MatchArm],
        idx: usize,
    ) -> Result<(), LowerError> {
        if idx >= arms.len() {
            // 到達不能（網羅性チェック済みの前提）
            ctx.emit(Instruction::Unreachable);
            return Ok(());
        }

        let arm = &arms[idx];

        match &arm.pattern {
            // ワイルドカードや変数パターンは常にマッチ
            Pattern::Wildcard(_) => {
                self.lower_expr(ctx, &arm.body)?;
            }
            Pattern::Var(_, name) => {
                // scrutinee を変数に束縛
                ctx.emit(Instruction::LocalGet(scrut_local));
                let var_local = ctx.alloc_local(name.clone());
                ctx.emit(Instruction::LocalSet(var_local));
                self.lower_expr(ctx, &arm.body)?;
            }
            Pattern::Lit(_, Literal::Int(n)) => {
                // scrutinee == n なら本体を実行
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(*n));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, &arm.body)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Lit(_, Literal::Bool(b)) => {
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 }));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_expr(ctx, &arm.body)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Constructor(_, name, sub_pats) if sub_pats.is_empty() => {
                // 引数なしコンストラクタ: リニアメモリからバリアントタグを読み出して比較
                if idx == arms.len() - 1 {
                    // 最後の腕はデフォルトとして扱う
                    self.lower_expr(ctx, &arm.body)?;
                } else {
                    let tag = self.adt_variant_indices.get(name)
                        .map(|(_, tag)| *tag as i64)
                        .unwrap_or(idx as i64);
                    // scrutinee (タグ付き i64 ポインタ) からアドレスを取得して
                    // variant_tag を offset 4 から読み出す
                    // I32WrapI64 でアドレス部分 (下位32ビット) を取得
                    ctx.emit(Instruction::LocalGet(scrut_local));
                    super::emit_untag_pointer(&mut ctx.instructions);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    // i32 -> i64 に拡張してタグ値と比較
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::I64Const(tag));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::If(IrType::I64));
                    self.lower_expr(ctx, &arm.body)?;
                    ctx.emit(Instruction::Else);
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                    ctx.emit(Instruction::End);
                }
            }
            Pattern::Constructor(_, name, sub_pats) => {
                // 引数付きコンストラクタ: リニアメモリからバリアントタグとフィールドを読み出す
                let tag = self.adt_variant_indices.get(name)
                    .map(|(_, tag)| *tag as i64)
                    .unwrap_or(idx as i64);

                // variant_tag で分岐 (最後の腕でなければ)
                let need_branch = idx < arms.len() - 1;
                if need_branch {
                    // scrutinee をアンタグして variant_tag を offset 4 から読み出し
                    ctx.emit(Instruction::LocalGet(scrut_local));
                    super::emit_untag_pointer(&mut ctx.instructions);
                    ctx.emit(Instruction::I32Load { offset: 4 });
                    ctx.emit(Instruction::I64ExtendI32U);
                    ctx.emit(Instruction::I64Const(tag));
                    ctx.emit(Instruction::I64Eq);
                    ctx.emit(Instruction::If(IrType::I64));
                }

                // フィールドを I64Load で取り出してローカル変数に束縛
                // 毎回 scrut_local からアンタグしてメモリアクセスする
                // (i64 ローカルから I32WrapI64 でアドレスを取得)
                for (i, sub_pat) in sub_pats.iter().enumerate() {
                    if let Pattern::Var(_, var_name) = sub_pat {
                        ctx.emit(Instruction::LocalGet(scrut_local));
                        super::emit_untag_pointer(&mut ctx.instructions);
                        ctx.emit(Instruction::I64Load { offset: 8 + (i as u32) * 8 });
                        let var_local = ctx.alloc_local(var_name.clone());
                        ctx.emit(Instruction::LocalSet(var_local));
                    }
                }

                self.lower_expr(ctx, &arm.body)?;

                if need_branch {
                    ctx.emit(Instruction::Else);
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                    ctx.emit(Instruction::End);
                }
            }
            Pattern::RecordPat(_, type_name, field_pats) => {
                // レコードパターン: StructGet でフィールドを抽出
                for (field_name, field_pat) in field_pats {
                    if let Pattern::Var(_, var_name) = field_pat {
                        // フィールドインデックスを解決
                        let field_idx = self.resolve_field_index(type_name, field_name);
                        let gc_type_idx = self.record_type_indices.get(type_name.as_str()).copied();

                        ctx.emit(Instruction::LocalGet(scrut_local));
                        if let (Some(gc_idx), Some(f_idx)) = (gc_type_idx, field_idx) {
                            // GC 型が登録されている場合は StructGet を使用
                            ctx.emit(Instruction::StructGet(gc_idx, f_idx));
                        }
                        // StructGet 結果（または scrutinee 自体）をローカルに格納
                        let var_local = ctx.alloc_local(var_name.clone());
                        ctx.emit(Instruction::LocalSet(var_local));
                    }
                }
                self.lower_expr(ctx, &arm.body)?;
            }
            _ => {
                return Err(LowerError::Unsupported {
                    msg: format!("パターン: {:?}", arm.pattern),
                });
            }
        }

        Ok(())
    }
}
