use std::collections::HashMap;

use super::{ModuleNode, scc};

#[test]
fn scc_helper_returns_stable_dependency_first_groups() {
    let modules = HashMap::from([
        (
            "Consumer".to_string(),
            ModuleNode {
                name: "Consumer".to_string(),
                imports: vec!["CycleA".to_string()],
                file_path: None,
            },
        ),
        (
            "CycleB".to_string(),
            ModuleNode {
                name: "CycleB".to_string(),
                imports: vec!["Base".to_string(), "CycleA".to_string()],
                file_path: None,
            },
        ),
        (
            "CycleA".to_string(),
            ModuleNode {
                name: "CycleA".to_string(),
                imports: vec!["CycleB".to_string(), "Base".to_string()],
                file_path: None,
            },
        ),
        (
            "Base".to_string(),
            ModuleNode {
                name: "Base".to_string(),
                imports: Vec::new(),
                file_path: None,
            },
        ),
    ]);

    assert_eq!(
        scc::compute_groups(&modules),
        vec![
            vec!["Base".to_string()],
            vec!["CycleA".to_string(), "CycleB".to_string()],
            vec!["Consumer".to_string()],
        ]
    );
}
