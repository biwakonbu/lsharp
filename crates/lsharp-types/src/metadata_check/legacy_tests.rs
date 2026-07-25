use super::{MetadataDiagnostic, Severity};

#[test]
fn legacy_invariant_module_exposes_type_probe() {
    let program = lsharp_syntax::parse("(defn bad [x] :invariant (+ x 1) x)").unwrap();
    let all_names = vec!["bad".to_string()];
    let diagnostics = super::legacy::check_legacy_invariant_types(&program, &all_names);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].severity, Severity::Error);
    assert!(diagnostics[0].message.contains(":invariant"));
    let _: &MetadataDiagnostic = &diagnostics[0];
}
