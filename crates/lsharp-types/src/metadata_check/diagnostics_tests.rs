use lsharp_syntax::ast::Decl;

use super::{MetadataDiagnostic, Severity};

#[test]
fn diagnostics_module_exposes_defn_metadata_check() {
    let program =
        lsharp_syntax::parse(r#"(defn add [x] :params [(missing "unknown")] x)"#).unwrap();
    let Decl::Defn {
        name,
        params,
        metadata: Some(metadata),
        span,
        ..
    } = &program.decls[0]
    else {
        panic!("expected a definition with metadata");
    };

    let mut diagnostics: Vec<MetadataDiagnostic> = Vec::new();
    super::diagnostics::check_defn_metadata(
        &mut diagnostics,
        name,
        params,
        metadata,
        *span,
        &["add".to_string()],
    );

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Error && diagnostic.message.contains("missing")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.severity == Severity::Warning && diagnostic.message.contains("x")
    }));
}
