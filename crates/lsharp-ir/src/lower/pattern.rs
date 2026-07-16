//! パターンマッチの lowering

use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::{Instruction, IrType};

use super::{FuncCtx, Lower, LowerError};

impl Lower {
    /// match の腕を if-else チェインに変換
    pub(crate) fn lower_match_arms(
        &mut self,
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
                self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
            }
            Pattern::Var(_, name) => {
                // scrutinee を変数に束縛
                ctx.emit(Instruction::LocalGet(scrut_local));
                let var_local = ctx.alloc_local(name.clone());
                ctx.emit(Instruction::LocalSet(var_local));
                self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
            }
            Pattern::Lit(_, Literal::Int(n)) => {
                // scrutinee == n なら本体を実行
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(*n));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Lit(_, Literal::Bool(b)) => {
                ctx.emit(Instruction::LocalGet(scrut_local));
                ctx.emit(Instruction::I64Const(if *b { 1 } else { 0 }));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
                ctx.emit(Instruction::Else);
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
            Pattern::Constructor(_, name, sub_pats) if sub_pats.is_empty() => {
                // 引数なしコンストラクタ: リニアメモリからバリアントタグを読み出して比較
                // BUG-3 修正: 最後の腕でもタグ比較を行う（デフォルト扱いしない）
                let tag = self
                    .adt_variant_indices
                    .get(name)
                    .map(|(_, tag)| *tag as i64)
                    .unwrap_or(idx as i64);
                ctx.emit(Instruction::LocalGet(scrut_local));
                super::emit_untag_pointer(&mut ctx.instructions);
                ctx.emit(Instruction::I32Load { offset: 4 });
                ctx.emit(Instruction::I64ExtendI32U);
                ctx.emit(Instruction::I64Const(tag));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
                ctx.emit(Instruction::Else);
                if idx == arms.len() - 1 {
                    ctx.emit(Instruction::Unreachable);
                } else {
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                }
                ctx.emit(Instruction::End);
            }
            Pattern::Constructor(_, name, sub_pats) => {
                // 引数付きコンストラクタ: リニアメモリからバリアントタグとフィールドを読み出す
                let tag = self
                    .adt_variant_indices
                    .get(name)
                    .map(|(_, tag)| *tag as i64)
                    .unwrap_or(idx as i64);

                // BUG-3 修正: 常にタグ比較を行う（最後の腕でもスキップしない）
                ctx.emit(Instruction::LocalGet(scrut_local));
                super::emit_untag_pointer(&mut ctx.instructions);
                ctx.emit(Instruction::I32Load { offset: 4 });
                ctx.emit(Instruction::I64ExtendI32U);
                ctx.emit(Instruction::I64Const(tag));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));

                // サブパターンを再帰的にバインド（ネストパターン対応）
                // ネストコンストラクタのタグチェックが失敗した場合、次の腕にフォールスルー
                self.bind_sub_patterns_with_fallback(ctx, scrut_local, sub_pats, arms, arm, idx)?;

                ctx.emit(Instruction::Else);
                if idx == arms.len() - 1 {
                    ctx.emit(Instruction::Unreachable);
                } else {
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                }
                ctx.emit(Instruction::End);
            }
            Pattern::RecordPat(_, type_name, field_pats) => {
                self.bind_record_pattern_fields(ctx, scrut_local, type_name, field_pats, arm.span)?;
                self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
            }
            _ => {
                return Err(LowerError::Unsupported {
                    msg: format!("パターン: {:?}", arm.pattern),
                    span: Some(arm.span),
                });
            }
        }

        Ok(())
    }

    /// レコードパターンの field を抽出し、nested record child を再帰的に bind する。
    fn bind_record_pattern_fields(
        &mut self,
        ctx: &mut FuncCtx,
        record_local: u32,
        type_name: &str,
        field_pats: &[(String, Pattern)],
        span: Span,
    ) -> Result<(), LowerError> {
        for (field_name, field_pat) in field_pats {
            let field_idx = self.resolve_field_index(type_name, field_name);
            let gc_type_idx = self.record_type_indices.get(type_name).copied();

            match field_pat {
                Pattern::Var(_, var_name) => {
                    ctx.emit(Instruction::LocalGet(record_local));
                    if let (Some(gc_idx), Some(f_idx)) = (gc_type_idx, field_idx) {
                        ctx.emit(Instruction::StructGet(gc_idx, f_idx));
                    }
                    let var_local = ctx.alloc_local(var_name.clone());
                    ctx.emit(Instruction::LocalSet(var_local));
                }
                Pattern::RecordPat(_, child_type_name, child_fields) => {
                    if let (Some(gc_idx), Some(f_idx)) = (gc_type_idx, field_idx) {
                        ctx.emit(Instruction::LocalGet(record_local));
                        ctx.emit(Instruction::StructGet(gc_idx, f_idx));
                        let temp_name = format!("__record_field_{}_{}", field_name, ctx.next_local);
                        let field_local = ctx.alloc_local(temp_name);
                        ctx.emit(Instruction::LocalSet(field_local));
                        self.bind_record_pattern_fields(
                            ctx,
                            field_local,
                            child_type_name,
                            child_fields,
                            span,
                        )?;
                    }
                }
                Pattern::Wildcard(_) => {}
                // literal/constructor child の判定は別の lowering 対応として残す。
                _ => {
                    return Err(LowerError::Unsupported {
                        msg: format!("レコードフィールドパターン: {:?}", field_pat),
                        span: Some(span),
                    });
                }
            }
        }
        Ok(())
    }

    /// ガード条件がある場合はガード式を評価して分岐、なければ直接 body を実行
    fn lower_arm_body_with_guard(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        arms: &[MatchArm],
        arm: &MatchArm,
        idx: usize,
    ) -> Result<(), LowerError> {
        if let Some(guard) = &arm.guard {
            // ガード式を評価 (Bool -> i64 の 0/1)
            self.lower_expr(ctx, guard)?;
            // i64 -> i32 に変換 (Wasm の if は i32 を消費する)
            ctx.emit(Instruction::I32WrapI64);
            ctx.emit(Instruction::If(IrType::I64));
            self.lower_expr(ctx, &arm.body)?;
            ctx.emit(Instruction::Else);
            // ガード条件不成立 -> 次の腕にフォールスルー
            self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
            ctx.emit(Instruction::End);
        } else {
            self.lower_expr(ctx, &arm.body)?;
        }
        Ok(())
    }

    /// コンストラクタのサブパターンを再帰的にバインド（ネストパターンのタグチェック付き）
    /// ネストしたコンストラクタパターンのタグが不一致の場合、次の腕にフォールスルーする
    fn bind_sub_patterns_with_fallback(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        sub_pats: &[Pattern],
        arms: &[MatchArm],
        arm: &MatchArm,
        idx: usize,
    ) -> Result<(), LowerError> {
        // ネストコンストラクタパターンがあるかチェック
        let has_nested_constructor = sub_pats
            .iter()
            .any(|p| matches!(p, Pattern::Constructor(_, _, _)));

        if has_nested_constructor {
            // ネストコンストラクタを含む場合:
            // 各フィールドを一時変数に取り出し、ネストコンストラクタのタグを全てチェック
            // 全てマッチしたら変数束縛 + body 実行
            // いずれかが不一致なら次の腕にフォールスルー

            // 1. フィールドを一時変数に取り出す
            // 親ローカルは scrut_local（外側のコンストラクタのタグチェック済み）
            let mut field_locals = Vec::new();
            for (i, _sub_pat) in sub_pats.iter().enumerate() {
                let temp_name = format!("__field_{}_{}", i, ctx.next_local);
                ctx.emit(Instruction::LocalGet(scrut_local));
                super::emit_untag_pointer(&mut ctx.instructions);
                ctx.emit(Instruction::I64Load {
                    offset: 8 + (i as u32) * 8,
                });
                let temp_local = ctx.alloc_local(temp_name);
                ctx.emit(Instruction::LocalSet(temp_local));
                field_locals.push(temp_local);
            }

            // 2. ネストコンストラクタのタグチェック条件を積み上げ
            self.emit_nested_checks_and_bind(
                ctx,
                scrut_local,
                sub_pats,
                &field_locals,
                0,
                arms,
                arm,
                idx,
            )?;
        } else {
            // ネストコンストラクタなし: 従来通りの処理
            self.bind_simple_sub_patterns(ctx, scrut_local, sub_pats)?;
            self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
        }

        Ok(())
    }

    /// ネストパターンのタグチェックを再帰的に行い、全てマッチしたら変数束縛 + body 実行
    #[allow(clippy::too_many_arguments)]
    fn emit_nested_checks_and_bind(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        sub_pats: &[Pattern],
        field_locals: &[u32],
        check_idx: usize,
        arms: &[MatchArm],
        arm: &MatchArm,
        idx: usize,
    ) -> Result<(), LowerError> {
        // 次のネストコンストラクタを見つける
        let next_nested = sub_pats
            .iter()
            .enumerate()
            .skip(check_idx)
            .find(|(_, p)| matches!(p, Pattern::Constructor(_, _, _)));

        if let Some((i, Pattern::Constructor(_, inner_name, inner_sub_pats))) = next_nested {
            let inner_tag = self
                .adt_variant_indices
                .get(inner_name)
                .map(|(_, tag)| *tag as i64)
                .unwrap_or(0);

            if inner_sub_pats.is_empty() {
                // 引数なしネストコンストラクタ: タグチェック
                ctx.emit(Instruction::LocalGet(field_locals[i]));
                super::emit_untag_pointer(&mut ctx.instructions);
                ctx.emit(Instruction::I32Load { offset: 4 });
                ctx.emit(Instruction::I64ExtendI32U);
                ctx.emit(Instruction::I64Const(inner_tag));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                // タグ一致: 次のネストチェックに進む
                self.emit_nested_checks_and_bind(
                    ctx,
                    scrut_local,
                    sub_pats,
                    field_locals,
                    i + 1,
                    arms,
                    arm,
                    idx,
                )?;
                ctx.emit(Instruction::Else);
                // タグ不一致: 次の腕にフォールスルー（scrut_local を使用）
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            } else {
                // 引数付きネストコンストラクタ: タグチェック + サブフィールドバインド
                ctx.emit(Instruction::LocalGet(field_locals[i]));
                super::emit_untag_pointer(&mut ctx.instructions);
                ctx.emit(Instruction::I32Load { offset: 4 });
                ctx.emit(Instruction::I64ExtendI32U);
                ctx.emit(Instruction::I64Const(inner_tag));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                // タグ一致: 内側のサブパターンをバインド
                self.bind_simple_sub_patterns(ctx, field_locals[i], inner_sub_pats)?;
                // 次のネストチェックに進む
                self.emit_nested_checks_and_bind(
                    ctx,
                    scrut_local,
                    sub_pats,
                    field_locals,
                    i + 1,
                    arms,
                    arm,
                    idx,
                )?;
                ctx.emit(Instruction::Else);
                // タグ不一致: 次の腕にフォールスルー（scrut_local を使用）
                self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                ctx.emit(Instruction::End);
            }
        } else {
            // 全てのネストチェック完了: 非ネストのサブパターンをバインドして body 実行
            for (i, sub_pat) in sub_pats.iter().enumerate() {
                match sub_pat {
                    Pattern::Var(_, var_name) => {
                        ctx.emit(Instruction::LocalGet(field_locals[i]));
                        let var_local = ctx.alloc_local(var_name.clone());
                        ctx.emit(Instruction::LocalSet(var_local));
                    }
                    Pattern::Wildcard(_) | Pattern::Constructor(_, _, _) => {
                        // ワイルドカード・コンストラクタ（既にチェック済み）: スキップ
                    }
                    _ => {}
                }
            }
            self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx)?;
        }

        Ok(())
    }

    /// 単純なサブパターン（Var/Wildcard のみ）をバインド
    fn bind_simple_sub_patterns(
        &mut self,
        ctx: &mut FuncCtx,
        parent_local: u32,
        sub_pats: &[Pattern],
    ) -> Result<(), LowerError> {
        for (i, sub_pat) in sub_pats.iter().enumerate() {
            match sub_pat {
                Pattern::Var(_, var_name) => {
                    ctx.emit(Instruction::LocalGet(parent_local));
                    super::emit_untag_pointer(&mut ctx.instructions);
                    ctx.emit(Instruction::I64Load {
                        offset: 8 + (i as u32) * 8,
                    });
                    let var_local = ctx.alloc_local(var_name.clone());
                    ctx.emit(Instruction::LocalSet(var_local));
                }
                Pattern::Wildcard(_) => {
                    // 何もしない
                }
                _ => {
                    // ネストパターンなどは bind_sub_patterns_with_fallback で処理済み
                }
            }
        }
        Ok(())
    }
}
