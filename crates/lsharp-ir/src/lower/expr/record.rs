use std::collections::HashMap;

use lsharp_syntax::ast::Expr;
use lsharp_syntax::span::Span;

use crate::lower::{FuncCtx, Lower, LowerBackend, LowerError};
use crate::{Instruction, IrType};

impl Lower {
    pub(super) fn lower_record_lit(
        &mut self,
        ctx: &mut FuncCtx,
        type_name: &str,
        fields: &[(String, Expr)],
    ) -> Result<(), LowerError> {
        if let Some(&gc_type_idx) = self.record_type_indices.get(type_name) {
            // レコード定義のフィールド順序に従って値をスタックに積む
            if let Some(field_order) = self.record_fields.get(type_name).cloned() {
                let field_map: HashMap<&str, &Expr> =
                    fields.iter().map(|(n, e)| (n.as_str(), e)).collect();
                for field_name in &field_order {
                    if let Some(expr) = field_map.get(field_name.as_str()) {
                        self.lower_expr(ctx, expr)?;
                    } else {
                        // フィールドが見つからない場合はデフォルト値
                        ctx.emit(Instruction::I64Const(0));
                    }
                }
            } else {
                // フィールド順序不明の場合は指定順に積む
                for (_, field_expr) in fields {
                    self.lower_expr(ctx, field_expr)?;
                }
            }
            ctx.emit(Instruction::StructNew(gc_type_idx));
        } else {
            // GC 型が見つからない場合はフォールバック
            if let Some((_, first_field)) = fields.first() {
                self.lower_expr(ctx, first_field)?;
            } else {
                ctx.emit(Instruction::I64Const(0));
            }
        }
        Ok(())
    }

    pub(super) fn lower_field_access(
        &mut self,
        ctx: &mut FuncCtx,
        expr_span: Span,
        expr: &Expr,
        field_name: &str,
    ) -> Result<(), LowerError> {
        // 式を評価してスタックにレコード値を積む
        self.lower_expr(ctx, expr)?;

        // 型推論結果から型名を取得して正確にフィールドを解決 (R-M5)
        let type_name_hint = self.infer_expr_type_name(expr);
        let mut resolved = false;

        if let Some(ref tn) = type_name_hint {
            // 型名が判明: 正確に解決
            if let Some(fields) = self.record_fields.get(tn).cloned() {
                if let Some(field_idx) = fields.iter().position(|f| f == field_name) {
                    if let Some(&gc_type_idx) = self.record_type_indices.get(tn) {
                        ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                        resolved = true;
                    }
                } else {
                    return Err(LowerError::Unsupported {
                        msg: format!(
                            "レコード型 '{tn}' にフィールド '{field_name}' が存在しません"
                        ),
                        span: Some(expr_span),
                    });
                }
            }
        }

        if !resolved {
            // フォールバック: フィールド名で全レコード型を走査
            // record_fields を一時的にクローンして借用問題を回避
            let record_fields_snapshot: Vec<(String, Vec<String>)> = self
                .record_fields
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            for (type_name, fields) in &record_fields_snapshot {
                if let Some(field_idx) = fields.iter().position(|f| f == field_name)
                    && let Some(&gc_type_idx) = self.record_type_indices.get(type_name)
                {
                    ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                    resolved = true;
                    break;
                }
            }
        }

        if !resolved {
            return Err(LowerError::Unsupported {
                msg: format!("フィールド '{field_name}' を解決できません"),
                span: Some(expr_span),
            });
        }
        Ok(())
    }

    pub(super) fn lower_record_update(
        &mut self,
        ctx: &mut FuncCtx,
        base: &Expr,
        update_fields: &[(String, Expr)],
    ) -> Result<(), LowerError> {
        // ベースレコードを評価してローカルに保存
        self.lower_expr(ctx, base)?;
        let base_type_name = self.infer_expr_type_name_with_ctx(ctx, base);
        let base_ir_type = base_type_name
            .as_deref()
            .and_then(|type_name| {
                (self.backend == LowerBackend::WasmGc)
                    .then(|| {
                        self.record_type_indices
                            .get(type_name)
                            .copied()
                            .map(IrType::Ref)
                    })
                    .flatten()
            })
            .unwrap_or(IrType::I64);
        let base_local = ctx.alloc_local_typed("_record_base".to_string(), base_ir_type);
        ctx.emit(Instruction::LocalSet(base_local));

        // 型推論結果からベース式の型名を取得 (R-m3)
        let type_name_hint = base_type_name;
        let mut found_type = None;

        if let Some(ref tn) = type_name_hint {
            // 型名が判明: 正確に解決
            if let Some(fields) = self.record_fields.get(tn).cloned() {
                found_type = Some((tn.clone(), fields));
            }
        }

        if found_type.is_none() {
            // フォールバック: フィールド名で全レコード型を走査
            let record_fields_snapshot: Vec<(String, Vec<String>)> = self
                .record_fields
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            for (type_name, fields) in record_fields_snapshot {
                let all_match = update_fields.iter().all(|(n, _)| fields.contains(n));
                if all_match {
                    found_type = Some((type_name, fields));
                    break;
                }
            }
        }

        if let Some((type_name, field_order)) = found_type {
            if let Some(&gc_type_idx) = self.record_type_indices.get(&type_name) {
                let update_map: HashMap<&str, &Expr> =
                    update_fields.iter().map(|(n, e)| (n.as_str(), e)).collect();
                // 各フィールドについて、更新値があればそれを、なければベースから取得
                for (field_idx, field_name) in field_order.iter().enumerate() {
                    if let Some(expr) = update_map.get(field_name.as_str()) {
                        self.lower_expr(ctx, expr)?;
                    } else {
                        ctx.emit(Instruction::LocalGet(base_local));
                        ctx.emit(Instruction::StructGet(gc_type_idx, field_idx as u32));
                    }
                }
                ctx.emit(Instruction::StructNew(gc_type_idx));
            } else {
                ctx.emit(Instruction::LocalGet(base_local));
            }
        } else {
            // フォールバック: ベースをそのまま返す
            ctx.emit(Instruction::LocalGet(base_local));
        }
        Ok(())
    }
}
