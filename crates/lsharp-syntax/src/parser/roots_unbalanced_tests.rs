//! `:roots-unbalanced "<理由>"` directive のパース検査。
//! 判断は docs/adr/decisions-root-lifetime-intentional-imbalance-annotation.md が正本。

use crate::ast::Decl;
use crate::parse;

fn defn_metadata(source: &str) -> crate::ast::Metadata {
    let prog = parse(source).unwrap();
    match &prog.decls[0] {
        Decl::Defn { metadata, .. } => metadata.clone().expect("metadata が付くべき"),
        other => panic!("Defn を期待した: {other:?}"),
    }
}

#[test]
fn test_roots_unbalanced_metadata_carries_reason() {
    let m = defn_metadata(
        r#"(defn push-roots [n] :roots-unbalanced "grow 確認のため積み増したまま返る" n)"#,
    );
    assert_eq!(
        m.roots_unbalanced.as_deref(),
        Some("grow 確認のため積み増したまま返る"),
        "理由文字列をそのまま保持すべき"
    );
}

#[test]
fn test_roots_unbalanced_coexists_with_other_directives() {
    // metadata ループが Colon + Symbol で再入するため、前後に別 directive を置いても
    // 読み落としや誤読が起きないことを固定する。
    let m = defn_metadata(
        r#"(defn push-roots [n] :doc "説明" :roots-unbalanced "理由" :returns "n" n)"#,
    );
    assert_eq!(m.doc.as_deref(), Some("説明"));
    assert_eq!(m.roots_unbalanced.as_deref(), Some("理由"));
    assert_eq!(m.returns.as_deref(), Some("n"));
}

#[test]
fn test_metadata_without_roots_unbalanced_is_none() {
    let m = defn_metadata(r#"(defn f [n] :doc "説明" n)"#);
    assert_eq!(m.roots_unbalanced, None, "既定は注釈なしであるべき");
}
