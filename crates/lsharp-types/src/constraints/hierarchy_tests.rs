use super::*;
use crate::types::{ConstrainedTypeInfo, Type};

fn make_registry() -> HashMap<String, ConstrainedTypeInfo> {
    let mut registry = HashMap::new();

    // Natural: Int (>= 0)
    registry.insert(
        "Natural".to_string(),
        ConstrainedTypeInfo {
            name: "Natural".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Gte(0)],
        },
    );

    // Percentage: Natural (<= 100)
    // -> 継承後は (>= 0, <= 100)
    registry.insert(
        "Percentage".to_string(),
        ConstrainedTypeInfo {
            name: "Percentage".to_string(),
            base_type: Type::Con("Natural".to_string()),
            constraints: vec![ConstraintDef::Lte(100)],
        },
    );

    // Priority: Percentage (<= 10)
    // -> 継承後は (>= 0, <= 100, <= 10)
    registry.insert(
        "Priority".to_string(),
        ConstrainedTypeInfo {
            name: "Priority".to_string(),
            base_type: Type::Con("Percentage".to_string()),
            constraints: vec![ConstraintDef::Lte(10)],
        },
    );

    registry
}

#[test]
fn test_resolve_simple_hierarchy() {
    let registry = make_registry();
    let constraints = resolve_constraint_hierarchy("Natural", &registry);
    assert_eq!(constraints.len(), 1);
    assert!(matches!(constraints[0], ConstraintDef::Gte(0)));
}

#[test]
fn test_resolve_two_level_hierarchy() {
    let registry = make_registry();
    let constraints = resolve_constraint_hierarchy("Percentage", &registry);
    // Natural の (>= 0) + Percentage の (<= 100)
    assert_eq!(constraints.len(), 2);
    assert!(matches!(constraints[0], ConstraintDef::Gte(0)));
    assert!(matches!(constraints[1], ConstraintDef::Lte(100)));
}

#[test]
fn test_resolve_three_level_hierarchy() {
    let registry = make_registry();
    let constraints = resolve_constraint_hierarchy("Priority", &registry);
    // Natural (>= 0) + Percentage (<= 100) + Priority (<= 10)
    assert_eq!(constraints.len(), 3);
    assert!(matches!(constraints[0], ConstraintDef::Gte(0)));
    assert!(matches!(constraints[1], ConstraintDef::Lte(100)));
    assert!(matches!(constraints[2], ConstraintDef::Lte(10)));
}

#[test]
fn test_resolve_nonexistent_type() {
    let registry = make_registry();
    let constraints = resolve_constraint_hierarchy("Unknown", &registry);
    assert!(constraints.is_empty());
}

#[test]
fn test_resolve_base_type_not_constrained() {
    let mut registry = HashMap::new();
    // Port: Int (range 1 65535) - 基底型 Int は制約付き型ではない
    registry.insert(
        "Port".to_string(),
        ConstrainedTypeInfo {
            name: "Port".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::Range(1, 65535)],
        },
    );
    let constraints = resolve_constraint_hierarchy("Port", &registry);
    assert_eq!(constraints.len(), 1);
    assert!(matches!(constraints[0], ConstraintDef::Range(1, 65535)));
}

#[test]
fn test_compatibility_valid() {
    let parent = vec![ConstraintDef::Gte(0), ConstraintDef::Lte(200)];
    let child = vec![ConstraintDef::Gte(0), ConstraintDef::Lte(100)];
    let errors = check_constraint_compatibility(&parent, &child);
    assert!(errors.is_empty());
}

#[test]
fn test_compatibility_child_exceeds_parent_lower() {
    let parent = vec![ConstraintDef::Gte(0)];
    let child = vec![ConstraintDef::Gte(-10)];
    let errors = check_constraint_compatibility(&parent, &child);
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_compatibility_child_exceeds_parent_upper() {
    let parent = vec![ConstraintDef::Lte(100)];
    let child = vec![ConstraintDef::Lte(200)];
    let errors = check_constraint_compatibility(&parent, &child);
    assert_eq!(errors.len(), 1);
}

#[test]
fn test_compatibility_range_check() {
    let parent = vec![ConstraintDef::Range(0, 100)];
    let child = vec![ConstraintDef::Range(-5, 50)];
    let errors = check_constraint_compatibility(&parent, &child);
    assert_eq!(errors.len(), 1); // 子の下限が親の範囲外
}
