use std::collections::HashMap;

use super::hierarchy::{check_constraint_compatibility, resolve_constraint_hierarchy};
use crate::types::{ConstrainedTypeInfo, ConstraintDef};

/// 制約階層間の型変換情報
///
/// 2つの制約付き型間で安全な型変換が可能かを判定し、
/// 変換関数の情報を返す。
#[derive(Debug, Clone)]
pub struct ConversionInfo {
    /// 変換元の型名
    pub from_type: String,
    /// 変換先の型名
    pub to_type: String,
    /// 変換方向
    pub kind: ConversionKind,
    /// 追加で必要な制約チェック（アップキャスト時は不要、ダウンキャスト時は必要）
    pub extra_checks: Vec<ConstraintDef>,
}

/// 型変換の種別
#[derive(Debug, Clone, PartialEq)]
pub enum ConversionKind {
    /// 安全な拡大変換（子 -> 親: 追加チェック不要）
    Upcast,
    /// 制約付き縮小変換（親 -> 子: ランタイムチェック必要）
    Downcast,
    /// 変換不可
    Incompatible,
}

/// 2つの制約付き型間の変換情報を生成
pub fn analyze_conversion(
    from_type: &str,
    to_type: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
) -> ConversionInfo {
    let from_constraints = resolve_constraint_hierarchy(from_type, constrained_types);
    let to_constraints = resolve_constraint_hierarchy(to_type, constrained_types);

    // from が to の部分型か判定: from の制約が to の制約を全て包含（同等以上に厳しい）
    let from_subsumes_to = is_subtype_constraints(&from_constraints, &to_constraints);
    // to が from の部分型か判定
    let to_subsumes_from = is_subtype_constraints(&to_constraints, &from_constraints);

    if from_subsumes_to {
        // from は to の子型（より厳しい制約）→ to への変換はアップキャスト（安全）
        return ConversionInfo {
            from_type: from_type.to_string(),
            to_type: to_type.to_string(),
            kind: ConversionKind::Upcast,
            extra_checks: vec![],
        };
    }

    if to_subsumes_from {
        // to は from の子型（より厳しい制約）→ to への変換はダウンキャスト（チェック必要）
        let extra_checks: Vec<ConstraintDef> = to_constraints
            .iter()
            .filter(|c| !from_constraints.contains(c))
            .cloned()
            .collect();

        return ConversionInfo {
            from_type: from_type.to_string(),
            to_type: to_type.to_string(),
            kind: ConversionKind::Downcast,
            extra_checks,
        };
    }

    // どちらの方向も互換性なし
    ConversionInfo {
        from_type: from_type.to_string(),
        to_type: to_type.to_string(),
        kind: ConversionKind::Incompatible,
        extra_checks: vec![],
    }
}

/// child_constraints が parent_constraints の全制約を少なくとも同等に満たすか判定
///
/// 子型の条件: 親の各制約について、子が同等以上に厳しい制約を持つこと。
/// 例: 親が Gte(0) なら子は Gte(0) 以上を持つ必要がある。
fn is_subtype_constraints(child: &[ConstraintDef], parent: &[ConstraintDef]) -> bool {
    // 親の制約が空なら、どの子も部分型
    if parent.is_empty() {
        return true;
    }
    // 子の制約が空で親の制約があるなら、子は部分型ではない
    if child.is_empty() && !parent.is_empty() {
        return false;
    }

    // 互換性チェック（子が親の制約を違反しないか）
    let compat_errors = check_constraint_compatibility(parent, child);
    if !compat_errors.is_empty() {
        return false;
    }

    // 追加条件: 親の各制約の「種類」について、子が対応する制約を持つか確認
    for p in parent {
        let has_corresponding = child.iter().any(|c| constraints_same_dimension(p, c));
        if !has_corresponding {
            return false;
        }
    }

    true
}

/// 2つの制約が同じ次元（同じ種類のチェック）に属するか判定
fn constraints_same_dimension(a: &ConstraintDef, b: &ConstraintDef) -> bool {
    use ConstraintDef::*;
    matches!(
        (a, b),
        (Gte(_), Gte(_))
            | (Gte(_), Range(_, _))
            | (Lte(_), Lte(_))
            | (Lte(_), Range(_, _))
            | (Range(_, _), Range(_, _))
            | (Range(_, _), Gte(_))
            | (Range(_, _), Lte(_))
            | (OneOf(_), OneOf(_))
            | (MinLength(_), MinLength(_))
            | (MaxLength(_), MaxLength(_))
            | (Matches(_), Matches(_))
            | (Satisfies(_), Satisfies(_))
    )
}
