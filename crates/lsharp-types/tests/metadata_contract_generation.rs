//! legacy contract forms から runner test 計画への投影を pin する。
//!
//! 由来は `84ca54fd` (`refactor: generate tests from contract inventory`)。
//! 当該 commit は `generate_tests` そのものを inventory 経由へ差し替える設計だったが、
//! main は別経路 — `run_metadata_tests` が `inventory_contract_suites` を併走させて
//! fail-closed にする — を採ったため、パッチ本体は取り込まない。
//! **pin されるべき契約 2 つ** (投影の順序 / projection mismatch の fail-closed) だけを
//! 現行 API の上へ書き直して残す。

use lsharp_syntax::{ast::Decl, parse};
use lsharp_types::{
    metadata_check::{TestKind, generate_tests},
    metadata_contract::{ContractInventoryError, inventory_contract_suites},
};

const LEGACY_CONTRACT_FORMS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../lsharp-syntax/tests/fixtures/metadata/legacy_contract_forms.ls"
));

/// v0.1 互換の並び — invariant を先頭、example は source 順。
#[test]
fn legacy_contract_forms_project_invariant_first_then_examples_in_source_order() {
    let program =
        parse(LEGACY_CONTRACT_FORMS).expect("legacy metadata fixture は parse できるべき");

    let tests = generate_tests(&program);

    assert_eq!(tests.len(), 3);
    assert_eq!(tests[0].name, "succ_invariant");
    assert_eq!(tests[0].kind, TestKind::Invariant);
    assert_eq!(tests[1].name, "succ_example_0");
    assert_eq!(tests[1].kind, TestKind::Example);
    assert_eq!(tests[2].name, "succ_example_1");
    assert_eq!(tests[2].kind, TestKind::Example);
}

/// compatibility aggregate と ordered forms がずれたら fail-closed にする。
/// aggregate を正本として黙って採用しない、が `metadata_contract.rs` の設計判断。
#[test]
fn projection_mismatch_between_aggregate_and_ordered_forms_fails_closed() {
    let mut program =
        parse(LEGACY_CONTRACT_FORMS).expect("legacy metadata fixture は parse できるべき");
    metadata_mut(&mut program).example.pop();

    assert_eq!(
        inventory_contract_suites(&program).unwrap_err(),
        ContractInventoryError::ProjectionMismatch {
            owner: "succ".to_string(),
        }
    );
}

fn metadata_mut(program: &mut lsharp_syntax::ast::Program) -> &mut lsharp_syntax::ast::Metadata {
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &mut program.decls[0]
    else {
        panic!("fixture の先頭宣言は metadata 付き defn であるべき");
    };
    metadata
}
