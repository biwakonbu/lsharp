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
    registry.insert(
        "Percentage".to_string(),
        ConstrainedTypeInfo {
            name: "Percentage".to_string(),
            base_type: Type::Con("Natural".to_string()),
            constraints: vec![ConstraintDef::Lte(100)],
        },
    );

    // Email: String (min-length 5, matches "@")
    registry.insert(
        "Email".to_string(),
        ConstrainedTypeInfo {
            name: "Email".to_string(),
            base_type: Type::Con("String".to_string()),
            constraints: vec![
                ConstraintDef::MinLength(5),
                ConstraintDef::Matches("@".to_string()),
            ],
        },
    );

    // Status: Int (one-of [0, 1, 2])
    registry.insert(
        "Status".to_string(),
        ConstrainedTypeInfo {
            name: "Status".to_string(),
            base_type: Type::Con("Int".to_string()),
            constraints: vec![ConstraintDef::OneOf(vec![0, 1, 2])],
        },
    );

    registry
}

#[test]
fn test_runtime_checks_natural() {
    let registry = make_registry();
    let check = generate_runtime_checks("Natural", &registry);
    assert_eq!(check.type_name, "Natural");
    assert_eq!(check.conditions.len(), 1);
    assert!(matches!(check.conditions[0], RuntimeCondition::IntGte(0)));
}

#[test]
fn test_runtime_checks_percentage_inherits() {
    // Percentage は Natural の (>= 0) + 自身の (<= 100) を継承
    let registry = make_registry();
    let check = generate_runtime_checks("Percentage", &registry);
    assert_eq!(check.type_name, "Percentage");
    assert_eq!(check.conditions.len(), 2);
    assert!(matches!(check.conditions[0], RuntimeCondition::IntGte(0)));
    assert!(matches!(check.conditions[1], RuntimeCondition::IntLte(100)));
}

#[test]
fn test_runtime_checks_email_string_constraints() {
    let registry = make_registry();
    let check = generate_runtime_checks("Email", &registry);
    assert_eq!(check.type_name, "Email");
    assert_eq!(check.conditions.len(), 2);
    assert!(matches!(
        check.conditions[0],
        RuntimeCondition::StrMinLength(5)
    ));
    assert!(matches!(
        check.conditions[1],
        RuntimeCondition::StrMatches(_)
    ));
}

#[test]
fn test_runtime_checks_one_of() {
    let registry = make_registry();
    let check = generate_runtime_checks("Status", &registry);
    assert_eq!(check.conditions.len(), 1);
    match &check.conditions[0] {
        RuntimeCondition::IntOneOf(values) => {
            assert_eq!(values, &vec![0, 1, 2]);
        }
        _ => panic!("IntOneOf が期待される"),
    }
}

#[test]
fn test_runtime_checks_unknown_type() {
    let registry = make_registry();
    let check = generate_runtime_checks("Unknown", &registry);
    assert_eq!(check.type_name, "Unknown");
    assert!(check.conditions.is_empty());
}

#[test]
fn test_constraint_to_runtime_satisfies_returns_none() {
    let result = constraint_to_runtime_condition(&ConstraintDef::Satisfies("is-valid".to_string()));
    assert!(result.is_none());
}

#[test]
fn test_constraint_to_runtime_range() {
    let result = constraint_to_runtime_condition(&ConstraintDef::Range(1, 100));
    match result {
        Some(RuntimeCondition::IntRange(lo, hi)) => {
            assert_eq!(lo, 1);
            assert_eq!(hi, 100);
        }
        _ => panic!("IntRange が期待される"),
    }
}
