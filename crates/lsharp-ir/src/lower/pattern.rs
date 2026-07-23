//! パターンマッチの lowering

use lsharp_syntax::ast::*;
use lsharp_syntax::span::Span;

use crate::{GcTypeKind, Instruction, IrType};

use super::{FuncCtx, Lower, LowerError};

type WasmGcPatternCheck = (u32, Pattern, Option<String>);

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
                let var_type = ctx
                    .local_types
                    .get(scrut_local as usize)
                    .copied()
                    .unwrap_or(IrType::I64);
                let var_local = ctx.alloc_local_typed(name.clone(), var_type);
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
                if self.backend == super::LowerBackend::WasmGc {
                    return self.lower_wasmgc_constructor_arm(ctx, scrut_local, arms, idx);
                }
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
                if self.backend == super::LowerBackend::WasmGc {
                    return self.lower_wasmgc_constructor_arm(ctx, scrut_local, arms, idx);
                }
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
                if self.backend == super::LowerBackend::WasmGc {
                    return self.lower_wasmgc_record_pattern_arm(
                        ctx,
                        scrut_local,
                        type_name,
                        field_pats,
                        arms,
                        arm,
                        idx,
                    );
                }
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

    /// WasmGC レコードパターンを field 値の sequence として lowering する。
    ///
    /// `lower_wasmgc_pattern_sequence` は ADT の nested constructor/literal と同じく、
    /// field pattern が不一致なら同じ scrutinee の次の arm へ戻す。これにより、
    /// `[{Point x 42} ...] [_ ...]` が不一致を暗黙に受理したり trap で終わったりしない。
    #[allow(clippy::too_many_arguments)]
    fn lower_wasmgc_record_pattern_arm(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        type_name: &str,
        field_pats: &[(String, Pattern)],
        arms: &[MatchArm],
        arm: &MatchArm,
        idx: usize,
    ) -> Result<(), LowerError> {
        let checks = self.collect_wasmgc_record_pattern_checks(
            ctx,
            scrut_local,
            type_name,
            field_pats,
            arm.span,
        )?;
        self.lower_wasmgc_pattern_sequence(ctx, checks, arms, idx, scrut_local)
    }

    fn collect_wasmgc_record_pattern_checks(
        &self,
        ctx: &mut FuncCtx,
        record_local: u32,
        type_name: &str,
        field_pats: &[(String, Pattern)],
        span: Span,
    ) -> Result<Vec<WasmGcPatternCheck>, LowerError> {
        let Some(&gc_type_idx) = self.record_type_indices.get(type_name) else {
            return Err(LowerError::Unsupported {
                msg: format!("WasmGC record type を解決できません: {type_name}"),
                span: Some(span),
            });
        };
        let Some(GcTypeKind::Struct(fields)) = self
            .gc_types
            .get(gc_type_idx as usize)
            .map(|type_def| &type_def.kind)
        else {
            return Err(LowerError::Unsupported {
                msg: format!("WasmGC record type が struct ではありません: {type_name}"),
                span: Some(span),
            });
        };

        let mut checks = Vec::with_capacity(field_pats.len());
        for (field_name, pattern) in field_pats {
            let Some(field_idx) = self.resolve_field_index(type_name, field_name) else {
                return Err(LowerError::Unsupported {
                    msg: format!("WasmGC record field を解決できません: {type_name}.{field_name}"),
                    span: Some(span),
                });
            };
            let field_type = fields
                .get(field_idx as usize)
                .map(|field| field.ty)
                .unwrap_or(IrType::I64);
            ctx.emit(Instruction::LocalGet(record_local));
            ctx.emit(Instruction::StructGet(gc_type_idx, field_idx));
            let field_local = ctx.alloc_local_typed(
                format!("_wasmgc_record_pattern_field_{field_name}"),
                field_type,
            );
            ctx.emit(Instruction::LocalSet(field_local));
            let field_type_name = match field_type {
                IrType::Ref(type_index) => self
                    .gc_types
                    .get(type_index as usize)
                    .map(|type_def| type_def.name.clone()),
                _ => None,
            };
            checks.push((field_local, pattern.clone(), field_type_name));
        }
        Ok(checks)
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

    fn lower_wasmgc_constructor_arm(
        &mut self,
        ctx: &mut FuncCtx,
        scrut_local: u32,
        arms: &[MatchArm],
        idx: usize,
    ) -> Result<(), LowerError> {
        let arm = &arms[idx];
        let Pattern::Constructor(_, name, sub_pats) = &arm.pattern else {
            return Err(LowerError::Unsupported {
                msg: "WasmGC ADT pattern の内部形式が不正です".to_string(),
                span: Some(arm.span),
            });
        };

        let Some(&(gc_type_idx, tag)) = self.adt_variant_indices.get(name) else {
            return Err(LowerError::Unsupported {
                msg: format!("WasmGC ADT variant を解決できません: {name}"),
                span: Some(arm.span),
            });
        };

        ctx.emit(Instruction::LocalGet(scrut_local));
        ctx.emit(Instruction::StructGet(gc_type_idx, 0));
        ctx.emit(Instruction::I64Const(tag as i64));
        ctx.emit(Instruction::I64Eq);
        ctx.emit(Instruction::If(IrType::I64));

        let field_types = self
            .adt_variant_field_types
            .get(name)
            .cloned()
            .unwrap_or_default();
        let field_type_names = self
            .adt_variant_field_type_names
            .get(name)
            .cloned()
            .unwrap_or_default();
        let mut checks = Vec::with_capacity(sub_pats.len());
        for (field_idx, pattern) in sub_pats.iter().enumerate() {
            ctx.emit(Instruction::LocalGet(scrut_local));
            ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32 + 1));
            let field_type = field_types.get(field_idx).copied().unwrap_or(IrType::I64);
            let field_local =
                ctx.alloc_local_typed(format!("_wasmgc_pattern_field_{field_idx}"), field_type);
            ctx.emit(Instruction::LocalSet(field_local));
            checks.push((
                field_local,
                pattern.clone(),
                field_type_names.get(field_idx).cloned().flatten(),
            ));
        }
        self.lower_wasmgc_pattern_sequence(ctx, checks, arms, idx, scrut_local)?;

        ctx.emit(Instruction::Else);
        if idx == arms.len() - 1 {
            ctx.emit(Instruction::Unreachable);
        } else {
            self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
        }
        ctx.emit(Instruction::End);
        Ok(())
    }

    fn lower_wasmgc_pattern_sequence(
        &mut self,
        ctx: &mut FuncCtx,
        checks: Vec<WasmGcPatternCheck>,
        arms: &[MatchArm],
        idx: usize,
        scrut_local: u32,
    ) -> Result<(), LowerError> {
        let arm = &arms[idx];
        let Some((value_local, pattern, value_type_name)) = checks.first() else {
            return self.lower_arm_body_with_guard(ctx, scrut_local, arms, arm, idx);
        };
        let rest = checks.iter().skip(1).cloned().collect::<Vec<_>>();

        match pattern {
            Pattern::Var(_, name) => {
                ctx.emit(Instruction::LocalGet(*value_local));
                let value_type = ctx
                    .local_types
                    .get(*value_local as usize)
                    .copied()
                    .unwrap_or(IrType::I64);
                let var_local = ctx.alloc_local_typed(name.clone(), value_type);
                ctx.emit(Instruction::LocalSet(var_local));
                if let Some(type_name) = value_type_name {
                    ctx.local_type_names.insert(name.clone(), type_name.clone());
                }
                self.lower_wasmgc_pattern_sequence(ctx, rest, arms, idx, scrut_local)
            }
            Pattern::Wildcard(_) => {
                self.lower_wasmgc_pattern_sequence(ctx, rest, arms, idx, scrut_local)
            }
            Pattern::Constructor(_, name, sub_pats) => {
                let Some(&(gc_type_idx, tag)) = self.adt_variant_indices.get(name) else {
                    return Err(LowerError::Unsupported {
                        msg: format!("WasmGC ADT variant を解決できません: {name}"),
                        span: Some(arm.span),
                    });
                };
                ctx.emit(Instruction::LocalGet(*value_local));
                ctx.emit(Instruction::StructGet(gc_type_idx, 0));
                ctx.emit(Instruction::I64Const(tag as i64));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));

                let field_types = self
                    .adt_variant_field_types
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let field_type_names = self
                    .adt_variant_field_type_names
                    .get(name)
                    .cloned()
                    .unwrap_or_default();
                let mut nested_checks = Vec::with_capacity(sub_pats.len() + rest.len());
                for (field_idx, nested_pattern) in sub_pats.iter().enumerate() {
                    ctx.emit(Instruction::LocalGet(*value_local));
                    ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32 + 1));
                    let field_type = field_types.get(field_idx).copied().unwrap_or(IrType::I64);
                    let field_local = ctx
                        .alloc_local_typed(format!("_wasmgc_nested_field_{field_idx}"), field_type);
                    ctx.emit(Instruction::LocalSet(field_local));
                    nested_checks.push((
                        field_local,
                        nested_pattern.clone(),
                        field_type_names.get(field_idx).cloned().flatten(),
                    ));
                }
                nested_checks.extend(rest);
                self.lower_wasmgc_pattern_sequence(ctx, nested_checks, arms, idx, scrut_local)?;

                ctx.emit(Instruction::Else);
                if idx == arms.len() - 1 {
                    ctx.emit(Instruction::Unreachable);
                } else {
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                }
                ctx.emit(Instruction::End);
                Ok(())
            }
            Pattern::Lit(_, literal) => {
                let Some(expected) = (match literal {
                    Literal::Int(value) => Some(*value),
                    Literal::Bool(value) => Some(i64::from(*value)),
                    Literal::Unit => Some(0),
                    Literal::Float(_) | Literal::String(_) => None,
                }) else {
                    return Err(LowerError::Unsupported {
                        msg: "WasmGC ADT の nested/literal pattern".to_string(),
                        span: Some(arm.span),
                    });
                };
                ctx.emit(Instruction::LocalGet(*value_local));
                ctx.emit(Instruction::I64Const(expected));
                ctx.emit(Instruction::I64Eq);
                ctx.emit(Instruction::If(IrType::I64));
                self.lower_wasmgc_pattern_sequence(ctx, rest, arms, idx, scrut_local)?;
                ctx.emit(Instruction::Else);
                if idx == arms.len() - 1 {
                    ctx.emit(Instruction::Unreachable);
                } else {
                    self.lower_match_arms(ctx, scrut_local, arms, idx + 1)?;
                }
                ctx.emit(Instruction::End);
                Ok(())
            }
            Pattern::RecordPat(_, type_name, field_pats) => {
                let mut nested_checks = self.collect_wasmgc_record_pattern_checks(
                    ctx,
                    *value_local,
                    type_name,
                    field_pats,
                    arm.span,
                )?;
                nested_checks.extend(rest);
                self.lower_wasmgc_pattern_sequence(ctx, nested_checks, arms, idx, scrut_local)
            }
        }
    }
}
