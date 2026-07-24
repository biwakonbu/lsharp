//! 制約評価エンジン
//!
//! 制約付き型 (type-constrained) の制約を評価する。
//! コンパイル時にリテラル値の制約チェックを行い、
//! ランタイム検証コードの生成を支援する。

use crate::regex::simple_pattern_match as shared_pattern_match;
use crate::types::ConstraintDef;

/// 制約評価エラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConstraintError {
    #[error("制約違反: {constraint} (値: {value})")]
    Violation { constraint: String, value: String },

    #[error("型不一致: {expected} が期待されましたが {actual} が渡されました")]
    TypeMismatch { expected: String, actual: String },
}

/// 制約評価結果
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintResult {
    /// 制約を満たす
    Satisfied,
    /// 制約を満たさない
    Violated(String),
    /// コンパイル時に評価不能（ランタイムで検証が必要）
    Deferred,
}

/// 整数値に対して制約を評価
pub fn eval_int_constraint(value: i64, constraint: &ConstraintDef) -> ConstraintResult {
    match constraint {
        ConstraintDef::Gte(min) => {
            if value >= *min {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!("値 {value} は >= {min} を満たしません"))
            }
        }
        ConstraintDef::Lte(max) => {
            if value <= *max {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!("値 {value} は <= {max} を満たしません"))
            }
        }
        ConstraintDef::Range(lo, hi) => {
            if value >= *lo && value <= *hi {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!("値 {value} は範囲 [{lo}, {hi}] に含まれません"))
            }
        }
        ConstraintDef::OneOf(values) => {
            if values.contains(&value) {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!("値 {value} は許可された値リストに含まれません"))
            }
        }
        // 整数値に対して文字列制約は型不一致
        ConstraintDef::Matches(_) | ConstraintDef::MinLength(_) | ConstraintDef::MaxLength(_) => {
            ConstraintResult::Violated("整数値に文字列制約は適用できません".to_string())
        }
        // satisfies は常に実行時評価
        ConstraintDef::Satisfies(_) => ConstraintResult::Deferred,
    }
}

/// 文字列値に対して制約を評価
pub fn eval_string_constraint(value: &str, constraint: &ConstraintDef) -> ConstraintResult {
    match constraint {
        ConstraintDef::MinLength(min) => {
            if value.len() >= *min {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "文字列長 {} は最小長 {min} を満たしません",
                    value.len()
                ))
            }
        }
        ConstraintDef::MaxLength(max) => {
            if value.len() <= *max {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "文字列長 {} は最大長 {max} を超えています",
                    value.len()
                ))
            }
        }
        ConstraintDef::Matches(pattern) => {
            if shared_pattern_match(value, pattern) {
                ConstraintResult::Satisfied
            } else {
                ConstraintResult::Violated(format!(
                    "文字列 \"{value}\" はパターン \"{pattern}\" にマッチしません"
                ))
            }
        }
        // 文字列値に対して整数制約は型不一致
        ConstraintDef::Gte(_)
        | ConstraintDef::Lte(_)
        | ConstraintDef::Range(_, _)
        | ConstraintDef::OneOf(_) => {
            ConstraintResult::Violated("文字列値に整数制約は適用できません".to_string())
        }
        ConstraintDef::Satisfies(_) => ConstraintResult::Deferred,
    }
}

/// 複数の制約を全て評価
pub fn eval_int_constraints(value: i64, constraints: &[ConstraintDef]) -> Vec<ConstraintResult> {
    constraints
        .iter()
        .map(|c| eval_int_constraint(value, c))
        .collect()
}

/// 複数の文字列制約を全て評価
pub fn eval_string_constraints(
    value: &str,
    constraints: &[ConstraintDef],
) -> Vec<ConstraintResult> {
    constraints
        .iter()
        .map(|c| eval_string_constraint(value, c))
        .collect()
}

/// 全制約が満たされているか
pub fn all_satisfied(results: &[ConstraintResult]) -> bool {
    results
        .iter()
        .all(|r| matches!(r, ConstraintResult::Satisfied))
}

/// 違反した制約のメッセージを収集
pub fn collect_violations(results: &[ConstraintResult]) -> Vec<String> {
    results
        .iter()
        .filter_map(|r| {
            if let ConstraintResult::Violated(msg) = r {
                Some(msg.clone())
            } else {
                None
            }
        })
        .collect()
}

/// 境界値テストケースの自動生成（整数制約用）
pub fn generate_boundary_test_cases(constraints: &[ConstraintDef]) -> Vec<(i64, bool)> {
    let mut cases = Vec::new();

    for constraint in constraints {
        match constraint {
            ConstraintDef::Gte(min) => {
                cases.push((*min, true)); // 境界: ちょうど min
                cases.push((*min - 1, false)); // 境界外: min - 1
                cases.push((*min + 1, true)); // 境界内: min + 1
            }
            ConstraintDef::Lte(max) => {
                cases.push((*max, true)); // 境界: ちょうど max
                cases.push((*max + 1, false)); // 境界外: max + 1
                cases.push((*max - 1, true)); // 境界内: max - 1
            }
            ConstraintDef::Range(lo, hi) => {
                cases.push((*lo, true)); // 下限境界
                cases.push((*lo - 1, false)); // 下限外
                cases.push((*hi, true)); // 上限境界
                cases.push((*hi + 1, false)); // 上限外
                if lo < hi {
                    cases.push(((*lo + *hi) / 2, true)); // 中間値
                }
            }
            ConstraintDef::OneOf(values) => {
                for &v in values {
                    cases.push((v, true));
                }
                // リストにない値
                if let Some(&max) = values.iter().max() {
                    cases.push((max + 1000, false));
                }
            }
            _ => {} // 文字列制約・satisfies はスキップ
        }
    }

    cases
}

use crate::types::ConstrainedTypeInfo;
use std::collections::HashMap;

/// 制約階層の解決
///
/// 制約付き型の基底型が別の制約付き型の場合、
/// 基底型の制約も自動的に継承する。
///
/// 例: Natural (>= 0), Percentage base=Natural (:constraints [(<= 100)])
/// -> Percentage は (>= 0) と (<= 100) の両方の制約を持つ
pub fn resolve_constraint_hierarchy(
    type_name: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
) -> Vec<ConstraintDef> {
    let mut all_constraints = Vec::new();
    let mut visited = Vec::new();
    collect_constraints_recursive(
        type_name,
        constrained_types,
        &mut all_constraints,
        &mut visited,
    );
    all_constraints
}

/// 再帰的に基底型の制約を収集
fn collect_constraints_recursive(
    type_name: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
    constraints: &mut Vec<ConstraintDef>,
    visited: &mut Vec<String>,
) {
    // 循環参照検出
    if visited.contains(&type_name.to_string()) {
        return;
    }
    visited.push(type_name.to_string());

    if let Some(info) = constrained_types.get(type_name) {
        // 基底型が別の制約付き型か確認
        let base_name = match &info.base_type {
            crate::types::Type::Con(name) => Some(name.clone()),
            _ => None,
        };

        // 基底型の制約を先に収集（階層の上位が先）
        if let Some(ref base) = base_name {
            collect_constraints_recursive(base, constrained_types, constraints, visited);
        }

        // 自身の制約を追加
        constraints.extend(info.constraints.clone());
    }
}

/// 制約の互換性チェック
///
/// 子の制約が親の制約を満たすか検証する。
/// 例: 親 (>= 0, <= 200), 子 (>= 0, <= 100) -> OK (子は親の範囲内)
/// 例: 親 (>= 0, <= 100), 子 (>= -10, <= 100) -> NG (子が親の下限を超える)
pub fn check_constraint_compatibility(
    parent_constraints: &[ConstraintDef],
    child_constraints: &[ConstraintDef],
) -> Vec<String> {
    let mut errors = Vec::new();

    // 親の各制約に対して、子の制約が親を満たすか検証
    for parent in parent_constraints {
        match parent {
            ConstraintDef::Gte(parent_min) => {
                // 子の Gte が親の Gte 以上であるか
                let child_min = child_constraints.iter().find_map(|c| match c {
                    ConstraintDef::Gte(v) => Some(*v),
                    ConstraintDef::Range(lo, _) => Some(*lo),
                    _ => None,
                });
                if let Some(cm) = child_min
                    && cm < *parent_min
                {
                    errors.push(format!(
                        "子の下限 ({cm}) が親の下限 ({parent_min}) より小さい"
                    ));
                }
            }
            ConstraintDef::Lte(parent_max) => {
                // 子の Lte が親の Lte 以下であるか
                let child_max = child_constraints.iter().find_map(|c| match c {
                    ConstraintDef::Lte(v) => Some(*v),
                    ConstraintDef::Range(_, hi) => Some(*hi),
                    _ => None,
                });
                if let Some(cm) = child_max
                    && cm > *parent_max
                {
                    errors.push(format!(
                        "子の上限 ({cm}) が親の上限 ({parent_max}) より大きい"
                    ));
                }
            }
            ConstraintDef::Range(parent_lo, parent_hi) => {
                // 子の範囲が親の範囲内か（Range、Gte、Lte のいずれかから推定）
                let child_range = child_constraints.iter().find_map(|c| match c {
                    ConstraintDef::Range(lo, hi) => Some((*lo, *hi)),
                    _ => None,
                });
                if let Some((clo, chi)) = child_range {
                    if clo < *parent_lo {
                        errors.push(format!(
                            "子の下限 ({clo}) が親の範囲下限 ({parent_lo}) より小さい"
                        ));
                    }
                    if chi > *parent_hi {
                        errors.push(format!(
                            "子の上限 ({chi}) が親の範囲上限 ({parent_hi}) より大きい"
                        ));
                    }
                } else {
                    // Range がない場合、Gte/Lte から推定して範囲チェック
                    let child_lo = child_constraints.iter().find_map(|c| match c {
                        ConstraintDef::Gte(v) => Some(*v),
                        _ => None,
                    });
                    let child_hi = child_constraints.iter().find_map(|c| match c {
                        ConstraintDef::Lte(v) => Some(*v),
                        _ => None,
                    });
                    if let Some(clo) = child_lo
                        && clo < *parent_lo
                    {
                        errors.push(format!(
                            "子の下限 ({clo}) が親の範囲下限 ({parent_lo}) より小さい"
                        ));
                    }
                    if let Some(chi) = child_hi
                        && chi > *parent_hi
                    {
                        errors.push(format!(
                            "子の上限 ({chi}) が親の範囲上限 ({parent_hi}) より大きい"
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    errors
}

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

/// ランタイム検証用の制約チェック式を IR 風の擬似コードとして生成
///
/// 制約付き型のスマートコンストラクタ（Name.new）で使用される
/// ランタイム検証ロジックの記述。
#[derive(Debug, Clone)]
pub struct RuntimeCheck {
    /// チェック対象の型名
    pub type_name: String,
    /// チェック条件のリスト（全て AND で結合）
    pub conditions: Vec<RuntimeCondition>,
}

/// ランタイム検証条件
#[derive(Debug, Clone)]
pub enum RuntimeCondition {
    /// value >= threshold
    IntGte(i64),
    /// value <= threshold
    IntLte(i64),
    /// lo <= value <= hi
    IntRange(i64, i64),
    /// value が指定値のいずれかと一致
    IntOneOf(Vec<i64>),
    /// 文字列長が最小長以上
    StrMinLength(usize),
    /// 文字列長が最大長以下
    StrMaxLength(usize),
    /// パターンマッチ
    StrMatches(String),
}

/// 制約付き型からランタイムチェック情報を生成
pub fn generate_runtime_checks(
    type_name: &str,
    constrained_types: &HashMap<String, ConstrainedTypeInfo>,
) -> RuntimeCheck {
    let constraints = resolve_constraint_hierarchy(type_name, constrained_types);
    let conditions: Vec<RuntimeCondition> = constraints
        .iter()
        .filter_map(constraint_to_runtime_condition)
        .collect();

    RuntimeCheck {
        type_name: type_name.to_string(),
        conditions,
    }
}

/// ConstraintDef を RuntimeCondition に変換
fn constraint_to_runtime_condition(constraint: &ConstraintDef) -> Option<RuntimeCondition> {
    match constraint {
        ConstraintDef::Gte(v) => Some(RuntimeCondition::IntGte(*v)),
        ConstraintDef::Lte(v) => Some(RuntimeCondition::IntLte(*v)),
        ConstraintDef::Range(lo, hi) => Some(RuntimeCondition::IntRange(*lo, *hi)),
        ConstraintDef::OneOf(values) => Some(RuntimeCondition::IntOneOf(values.clone())),
        ConstraintDef::MinLength(v) => Some(RuntimeCondition::StrMinLength(*v)),
        ConstraintDef::MaxLength(v) => Some(RuntimeCondition::StrMaxLength(*v)),
        ConstraintDef::Matches(p) => Some(RuntimeCondition::StrMatches(p.clone())),
        ConstraintDef::Satisfies(_) => None, // satisfies は静的に変換不能
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod hierarchy_tests;

#[cfg(test)]
mod conversion_tests;

#[cfg(test)]
mod runtime_check_tests;
