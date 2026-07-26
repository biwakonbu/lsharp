use super::{
    check_assertion_non_vacuity, check_assertion_types, check_case_non_vacuity, check_case_types,
    check_property_non_vacuity, check_property_types,
};
use lsharp_syntax::ast::Program;

#[test]
fn canonical_contract_check_modules_preserve_empty_program_contract() {
    let program = Program { decls: Vec::new() };

    assert!(check_assertion_non_vacuity(&program).is_empty());
    assert!(check_case_non_vacuity(&program).is_empty());
    assert!(check_property_non_vacuity(&program).is_empty());
    assert!(check_assertion_types(&program, &[]).is_empty());
    assert!(check_case_types(&program, &[]).is_empty());
    assert!(check_property_types(&program, &[]).is_empty());
}
