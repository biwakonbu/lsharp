use crate::ast::Decl;
use crate::parse;

#[test]
fn test_transitions_metadata() {
    let prog = parse(
        r#"(defn open-door [door] :doc "Opens a door" :transitions [(Closed -> Open)] door)"#,
    )
    .unwrap();
    assert_eq!(prog.decls.len(), 1);
    if let Decl::Defn { metadata, .. } = &prog.decls[0] {
        let m = metadata.as_ref().unwrap();
        assert_eq!(m.transitions.len(), 1);
        assert_eq!(m.transitions[0], ("Closed".to_string(), "Open".to_string()));
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_multiple_transitions() {
    let prog =
        parse(r#"(defn toggle [state] :transitions [(Open -> Closed) (Closed -> Open)] state)"#)
            .unwrap();
    if let Decl::Defn { metadata, .. } = &prog.decls[0] {
        let m = metadata.as_ref().unwrap();
        assert_eq!(m.transitions.len(), 2);
        assert_eq!(m.transitions[0], ("Open".to_string(), "Closed".to_string()));
        assert_eq!(m.transitions[1], ("Closed".to_string(), "Open".to_string()));
    } else {
        panic!("Expected Defn");
    }
}

#[test]
fn test_no_transitions() {
    let prog = parse(r#"(defn add [x y] :doc "add" (+ x y))"#).unwrap();
    if let Decl::Defn { metadata, .. } = &prog.decls[0] {
        let m = metadata.as_ref().unwrap();
        assert!(m.transitions.is_empty());
    } else {
        panic!("Expected Defn");
    }
}
