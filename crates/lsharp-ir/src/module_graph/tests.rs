use super::*;

#[test]
fn test_empty_graph() {
    let graph = ModuleGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.topological_sort().unwrap(), Vec::<String>::new());
}

#[test]
fn test_single_module() {
    let mut graph = ModuleGraph::new();
    graph.add_module("Main".to_string(), vec![], None).unwrap();
    assert_eq!(graph.len(), 1);
    let order = graph.topological_sort().unwrap();
    assert_eq!(order, vec!["Main"]);
}

#[test]
fn test_linear_dependencies() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph
        .add_module("B".to_string(), vec!["A".to_string()], None)
        .unwrap();
    graph
        .add_module("C".to_string(), vec!["B".to_string()], None)
        .unwrap();

    let order = graph.topological_sort().unwrap();
    // A は B より前、B は C より前
    let pos_a = order.iter().position(|n| n == "A").unwrap();
    let pos_b = order.iter().position(|n| n == "B").unwrap();
    let pos_c = order.iter().position(|n| n == "C").unwrap();
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
}

#[test]
fn test_diamond_dependencies() {
    let mut graph = ModuleGraph::new();
    graph.add_module("Base".to_string(), vec![], None).unwrap();
    graph
        .add_module("Left".to_string(), vec!["Base".to_string()], None)
        .unwrap();
    graph
        .add_module("Right".to_string(), vec!["Base".to_string()], None)
        .unwrap();
    graph
        .add_module(
            "Top".to_string(),
            vec!["Left".to_string(), "Right".to_string()],
            None,
        )
        .unwrap();

    let order = graph.topological_sort().unwrap();
    let pos_base = order.iter().position(|n| n == "Base").unwrap();
    let pos_left = order.iter().position(|n| n == "Left").unwrap();
    let pos_right = order.iter().position(|n| n == "Right").unwrap();
    let pos_top = order.iter().position(|n| n == "Top").unwrap();
    assert!(pos_base < pos_left);
    assert!(pos_base < pos_right);
    assert!(pos_left < pos_top);
    assert!(pos_right < pos_top);
}

#[test]
fn test_reverse_dependency_closure_linear_chain() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph
        .add_module("B".to_string(), vec!["A".to_string()], None)
        .unwrap();
    graph
        .add_module("C".to_string(), vec!["B".to_string()], None)
        .unwrap();

    assert_eq!(graph.reverse_dependency_closure("A"), vec!["B", "C"]);
    assert_eq!(graph.reverse_dependency_closure("B"), vec!["C"]);
}

#[test]
fn test_reverse_dependency_closure_diamond() {
    let mut graph = ModuleGraph::new();
    graph.add_module("Base".to_string(), vec![], None).unwrap();
    graph
        .add_module("Left".to_string(), vec!["Base".to_string()], None)
        .unwrap();
    graph
        .add_module("Right".to_string(), vec!["Base".to_string()], None)
        .unwrap();
    graph
        .add_module(
            "Top".to_string(),
            vec!["Left".to_string(), "Right".to_string()],
            None,
        )
        .unwrap();

    assert_eq!(
        graph.reverse_dependency_closure("Base"),
        vec!["Left", "Right", "Top"]
    );
}

#[test]
fn test_reverse_dependency_closure_independent_module() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph
        .add_module("B".to_string(), vec!["A".to_string()], None)
        .unwrap();
    graph
        .add_module("Isolated".to_string(), vec![], None)
        .unwrap();

    assert_eq!(
        graph.reverse_dependency_closure("Isolated"),
        Vec::<String>::new()
    );
}

#[test]
fn test_compute_dirty_set_includes_reverse_dependents() {
    let mut graph = ModuleGraph::new();
    graph.add_module("Base".to_string(), vec![], None).unwrap();
    graph
        .add_module("Left".to_string(), vec!["Base".to_string()], None)
        .unwrap();
    graph
        .add_module("Right".to_string(), vec!["Base".to_string()], None)
        .unwrap();
    graph
        .add_module(
            "Top".to_string(),
            vec!["Left".to_string(), "Right".to_string()],
            None,
        )
        .unwrap();

    assert_eq!(
        graph.compute_dirty_set(&["Base".to_string()]),
        vec!["Base", "Left", "Right", "Top"]
    );
}

#[test]
fn test_diff_imports_reports_added_and_removed_modules() {
    let diff = ModuleGraph::diff_imports(
        &["Base".to_string(), "Left".to_string()],
        &["Left".to_string(), "Right".to_string()],
    );

    assert_eq!(diff.added, vec!["Right"]);
    assert_eq!(diff.removed, vec!["Base"]);
}

#[test]
fn test_update_module_imports_rebuilds_reverse_deps_and_topological_sort() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph.add_module("B".to_string(), vec![], None).unwrap();
    graph
        .add_module("C".to_string(), vec!["B".to_string()], None)
        .unwrap();

    graph
        .update_module_imports("B", vec!["A".to_string()])
        .unwrap();

    let order = graph.topological_sort().unwrap();
    let pos_a = order.iter().position(|n| n == "A").unwrap();
    let pos_b = order.iter().position(|n| n == "B").unwrap();
    let pos_c = order.iter().position(|n| n == "C").unwrap();
    assert!(pos_a < pos_b);
    assert!(pos_b < pos_c);
    assert_eq!(graph.reverse_dependency_closure("A"), vec!["B", "C"]);
}

#[test]
fn test_remove_module_updates_reverse_deps_and_dirty_set() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph
        .add_module("B".to_string(), vec!["A".to_string()], None)
        .unwrap();
    graph
        .add_module("C".to_string(), vec!["B".to_string()], None)
        .unwrap();

    assert!(graph.remove_module("C"));
    assert_eq!(graph.topological_sort().unwrap(), vec!["A", "B"]);
    assert_eq!(graph.reverse_dependency_closure("A"), vec!["B"]);
    assert_eq!(graph.compute_dirty_set(&["A".to_string()]), vec!["A", "B"]);
}

#[test]
fn test_compute_dirty_set_expands_formatter_trio_atomically() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module(FORMATTER_TRIO_EXPR.to_string(), vec![], None)
        .unwrap();
    graph
        .add_module(FORMATTER_TRIO_DECL.to_string(), vec![], None)
        .unwrap();
    graph
        .add_module(FORMATTER_TRIO_MAIN.to_string(), vec![], None)
        .unwrap();
    graph
        .add_module(
            "Consumer".to_string(),
            vec![FORMATTER_TRIO_MAIN.to_string()],
            None,
        )
        .unwrap();

    assert_eq!(
        graph.compute_dirty_set(&[FORMATTER_TRIO_DECL.to_string()]),
        vec![
            FORMATTER_TRIO_EXPR,
            FORMATTER_TRIO_DECL,
            FORMATTER_TRIO_MAIN,
            "Consumer",
        ]
    );
}

/// マルチファイル compile の Wasm 決定性: HashMap 走査順に依存しないこと
#[test]
fn test_topological_sort_stable_across_calls() {
    let mut graph = ModuleGraph::new();
    graph.add_module("Z".to_string(), vec![], None).unwrap();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph
        .add_module(
            "M".to_string(),
            vec!["Z".to_string(), "A".to_string()],
            None,
        )
        .unwrap();

    let o1 = graph.topological_sort().unwrap();
    let o2 = graph.topological_sort().unwrap();
    assert_eq!(o1, o2);
    // import 名ソートにより A → Z → M（M の依存先を辞書順に処理）
    assert_eq!(o1, vec!["A", "Z", "M"]);
}

#[test]
fn test_scc_groups_are_stable_and_dependency_first() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module("Consumer".to_string(), vec!["CycleA".to_string()], None)
        .unwrap();
    graph
        .add_module(
            "CycleB".to_string(),
            vec!["Base".to_string(), "CycleA".to_string()],
            None,
        )
        .unwrap();
    graph
        .add_module(
            "CycleA".to_string(),
            vec!["CycleB".to_string(), "Base".to_string()],
            None,
        )
        .unwrap();
    graph.add_module("Base".to_string(), vec![], None).unwrap();

    let groups = graph.scc_groups();
    assert_eq!(
        groups,
        vec![
            vec!["Base".to_string()],
            vec!["CycleA".to_string(), "CycleB".to_string()],
            vec!["Consumer".to_string()],
        ]
    );
    assert_eq!(groups, graph.scc_groups());
}

#[test]
fn test_cycle_diagnostic_is_stable_across_module_insertion_order() {
    fn build_graph(order: &[&str]) -> ModuleGraph {
        let mut graph = ModuleGraph::new();
        for name in order {
            let imports = match *name {
                "A" => vec!["B".to_string()],
                "B" => vec!["C".to_string()],
                "C" => vec!["A".to_string()],
                _ => Vec::new(),
            };
            graph
                .add_module((*name).to_string(), imports, None)
                .unwrap();
        }
        graph
    }

    let first = build_graph(&["A", "B", "C"])
        .detect_cycles()
        .expect("A -> B -> C -> A は循環として検出されるべき");
    let second = build_graph(&["C", "B", "A"])
        .detect_cycles()
        .expect("挿入順が違っても同じ循環が検出されるべき");

    assert_eq!(first, vec!["A", "B", "C", "A"]);
    assert_eq!(second, first);
}

#[test]
fn test_missing_import_diagnostics_are_stable_and_sorted() {
    fn build_graph(order: &[&str]) -> ModuleGraph {
        let mut graph = ModuleGraph::new();
        for name in order {
            let imports = match *name {
                "Main" => vec!["MissingB".to_string(), "MissingA".to_string()],
                "Other" => vec!["MissingC".to_string()],
                _ => Vec::new(),
            };
            graph
                .add_module((*name).to_string(), imports, None)
                .unwrap();
        }
        graph
    }

    let expected = vec![
        "モジュール 'MissingA' が見つかりません ('Main' からインポート)",
        "モジュール 'MissingB' が見つかりません ('Main' からインポート)",
        "モジュール 'MissingC' が見つかりません ('Other' からインポート)",
    ];
    let first: Vec<String> = build_graph(&["Other", "Main"])
        .check_imports()
        .into_iter()
        .map(|error| error.to_string())
        .collect();
    let second: Vec<String> = build_graph(&["Main", "Other"])
        .check_imports()
        .into_iter()
        .map(|error| error.to_string())
        .collect();

    assert_eq!(first, expected);
    assert_eq!(second, first);
}

#[test]
fn test_cyclic_dependency_detection() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module("A".to_string(), vec!["B".to_string()], None)
        .unwrap();
    graph
        .add_module("B".to_string(), vec!["A".to_string()], None)
        .unwrap();

    assert!(graph.detect_cycles().is_some());
    assert!(graph.topological_sort().is_err());
}

#[test]
fn test_three_way_cycle() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module("A".to_string(), vec!["B".to_string()], None)
        .unwrap();
    graph
        .add_module("B".to_string(), vec!["C".to_string()], None)
        .unwrap();
    graph
        .add_module("C".to_string(), vec!["A".to_string()], None)
        .unwrap();

    let cycle = graph.detect_cycles().unwrap();
    assert!(cycle.len() > 2); // 少なくとも 3 ノードの循環
}

#[test]
fn test_duplicate_module_error() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    let result = graph.add_module("A".to_string(), vec![], None);
    assert!(result.is_err());
}

#[test]
fn test_missing_import_check() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module("Main".to_string(), vec!["Missing".to_string()], None)
        .unwrap();

    let errors = graph.check_imports();
    assert_eq!(errors.len(), 1);
    assert!(matches!(
        &errors[0],
        ModuleGraphError::ModuleNotFound { name, .. } if name == "Missing"
    ));
}

#[test]
fn test_file_path_mapping() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module(
            "Utils".to_string(),
            vec![],
            Some("src/utils.ls".to_string()),
        )
        .unwrap();

    assert_eq!(graph.module_for_file("src/utils.ls"), Some("Utils"));
    assert_eq!(graph.module_for_file("src/other.ls"), None);
}
