use super::ModuleGraph;

#[test]
fn mutation_module_reports_stable_import_diff() {
    let diff = ModuleGraph::diff_imports(
        &["Base".to_string(), "Left".to_string()],
        &["Left".to_string(), "Right".to_string()],
    );

    assert_eq!(diff.added, vec!["Right"]);
    assert_eq!(diff.removed, vec!["Base"]);
}
