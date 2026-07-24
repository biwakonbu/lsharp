//! 制約評価エンジン
//!
//! 制約付き型 (type-constrained) の制約を評価する。
//! コンパイル時にリテラル値の制約チェックを行い、
//! ランタイム検証コードの生成を支援する。

mod conversion;
mod eval;
mod hierarchy;
mod runtime;

pub use conversion::{ConversionInfo, ConversionKind, analyze_conversion};
pub use eval::{
    ConstraintError, ConstraintResult, all_satisfied, collect_violations, eval_int_constraint,
    eval_int_constraints, eval_string_constraint, eval_string_constraints,
    generate_boundary_test_cases,
};
pub use hierarchy::{check_constraint_compatibility, resolve_constraint_hierarchy};
pub use runtime::{RuntimeCheck, RuntimeCondition, generate_runtime_checks};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod hierarchy_tests;

#[cfg(test)]
mod conversion_tests;

#[cfg(test)]
mod runtime_check_tests;
