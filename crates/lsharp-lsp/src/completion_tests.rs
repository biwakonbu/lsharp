use super::*;

#[test]
fn test_complete_returns_matching_keyword_and_function_symbols() {
    let source = "(defn helper [] 1)\n(defn main [] (he))";
    let items = complete(source, Position::new(1, 17), &[]);
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();

    assert!(labels.contains(&"helper"));
    assert!(
        !labels.contains(&"defn"),
        "prefix=he では無関係な keyword は返さないべき"
    );
}

#[test]
fn test_complete_returns_import_module_candidates() {
    let source = "(import He)";
    let items = complete(
        source,
        Position::new(0, 10),
        &[
            "Hello".to_string(),
            "Helpers".to_string(),
            "World".to_string(),
        ],
    );
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();

    assert!(labels.contains(&"Hello"));
    assert!(labels.contains(&"Helpers"));
    assert!(!labels.contains(&"World"));
}
