use std::collections::HashMap;

use crate::types::{ConstrainedTypeInfo, ConstraintDef};

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
