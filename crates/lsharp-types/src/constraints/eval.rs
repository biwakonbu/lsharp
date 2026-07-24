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
