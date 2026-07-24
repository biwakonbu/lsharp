use super::*;

#[test]
fn test_gte_satisfied() {
    assert_eq!(
        eval_int_constraint(5, &ConstraintDef::Gte(0)),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_gte_boundary() {
    assert_eq!(
        eval_int_constraint(0, &ConstraintDef::Gte(0)),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_gte_violated() {
    assert!(matches!(
        eval_int_constraint(-1, &ConstraintDef::Gte(0)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_lte_satisfied() {
    assert_eq!(
        eval_int_constraint(50, &ConstraintDef::Lte(100)),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_lte_violated() {
    assert!(matches!(
        eval_int_constraint(101, &ConstraintDef::Lte(100)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_range_satisfied() {
    assert_eq!(
        eval_int_constraint(50, &ConstraintDef::Range(0, 100)),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_range_violated() {
    assert!(matches!(
        eval_int_constraint(-1, &ConstraintDef::Range(0, 100)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_one_of_satisfied() {
    assert_eq!(
        eval_int_constraint(2, &ConstraintDef::OneOf(vec![1, 2, 3])),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_one_of_violated() {
    assert!(matches!(
        eval_int_constraint(4, &ConstraintDef::OneOf(vec![1, 2, 3])),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_min_length_satisfied() {
    assert_eq!(
        eval_string_constraint("hello", &ConstraintDef::MinLength(3)),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_min_length_violated() {
    assert!(matches!(
        eval_string_constraint("hi", &ConstraintDef::MinLength(3)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_max_length_satisfied() {
    assert_eq!(
        eval_string_constraint("hi", &ConstraintDef::MaxLength(10)),
        ConstraintResult::Satisfied
    );
}

#[test]
fn test_max_length_violated() {
    assert!(matches!(
        eval_string_constraint("hello world", &ConstraintDef::MaxLength(5)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_satisfies_deferred() {
    assert_eq!(
        eval_int_constraint(42, &ConstraintDef::Satisfies("is-even".to_string())),
        ConstraintResult::Deferred
    );
}

#[test]
fn test_multiple_constraints() {
    let constraints = vec![ConstraintDef::Gte(0), ConstraintDef::Lte(100)];
    let results = eval_int_constraints(50, &constraints);
    assert!(all_satisfied(&results));
}

#[test]
fn test_multiple_constraints_violated() {
    let constraints = vec![ConstraintDef::Gte(0), ConstraintDef::Lte(100)];
    let results = eval_int_constraints(101, &constraints);
    assert!(!all_satisfied(&results));
    let violations = collect_violations(&results);
    assert_eq!(violations.len(), 1);
}

#[test]
fn test_boundary_test_generation() {
    let constraints = vec![ConstraintDef::Range(0, 100)];
    let cases = generate_boundary_test_cases(&constraints);
    // 5 ケース: lo, lo-1, hi, hi+1, mid
    assert_eq!(cases.len(), 5);

    // 境界値が含まれているか確認
    assert!(cases.iter().any(|(v, ok)| *v == 0 && *ok));
    assert!(cases.iter().any(|(v, ok)| *v == -1 && !*ok));
    assert!(cases.iter().any(|(v, ok)| *v == 100 && *ok));
    assert!(cases.iter().any(|(v, ok)| *v == 101 && !*ok));
}

#[test]
fn test_type_mismatch_int_string_constraint() {
    assert!(matches!(
        eval_int_constraint(42, &ConstraintDef::MinLength(3)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_type_mismatch_string_int_constraint() {
    assert!(matches!(
        eval_string_constraint("hello", &ConstraintDef::Gte(0)),
        ConstraintResult::Violated(_)
    ));
}

#[test]
fn test_string_constraint_uses_shared_regex_extended_features() {
    assert_eq!(
        eval_string_constraint(
            "abc123",
            &ConstraintDef::Matches("^\\w{3}\\d{3}$".to_string())
        ),
        ConstraintResult::Satisfied
    );
    assert!(matches!(
        eval_string_constraint(
            "ab123",
            &ConstraintDef::Matches("^\\w{3}\\d{3}$".to_string())
        ),
        ConstraintResult::Violated(_)
    ));
}
