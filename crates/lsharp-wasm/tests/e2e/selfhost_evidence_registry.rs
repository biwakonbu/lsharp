//! selfhost evidence registry の E2E contract tests。

#[test]
fn selfhost_evidence_registry_root_stays_within_the_test_file_budget() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("e2e")
        .join("selfhost_evidence_registry.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} を読み込めません: {error}", path.display()));
    let lines = source.lines().count();

    assert!(
        lines <= 500,
        "selfhost_evidence_registry.rs は責務別 module へ分割し、500 行以下に保つ (actual={lines})"
    );
}

#[path = "selfhost_evidence_registry/harness.rs"]
mod harness;
#[path = "selfhost_evidence_registry/identity.rs"]
mod identity;
#[path = "selfhost_evidence_registry/runtime.rs"]
mod runtime;
#[path = "selfhost_evidence_registry/validation.rs"]
mod validation;
