use std::collections::HashMap;

use super::hierarchy::resolve_constraint_hierarchy;
use crate::types::{ConstrainedTypeInfo, ConstraintDef};

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
pub(super) fn constraint_to_runtime_condition(
    constraint: &ConstraintDef,
) -> Option<RuntimeCondition> {
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
