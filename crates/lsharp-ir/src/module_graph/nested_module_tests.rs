use super::*;

#[test]
fn test_nested_module_name() {
    let mut graph = ModuleGraph::new();
    graph.add_module("App".to_string(), vec![], None).unwrap();
    graph
        .add_module("App.Utils".to_string(), vec![], None)
        .unwrap();
    graph
        .add_module(
            "App.Models".to_string(),
            vec!["App.Utils".to_string()],
            None,
        )
        .unwrap();

    let order = graph.topological_sort().unwrap();
    let pos_utils = order.iter().position(|n| n == "App.Utils").unwrap();
    let pos_models = order.iter().position(|n| n == "App.Models").unwrap();
    assert!(pos_utils < pos_models);
}

#[test]
fn test_nested_module_depth() {
    let mut graph = ModuleGraph::new();
    graph.add_module("A".to_string(), vec![], None).unwrap();
    graph
        .add_module("A.B".to_string(), vec!["A".to_string()], None)
        .unwrap();
    graph
        .add_module("A.B.C".to_string(), vec!["A.B".to_string()], None)
        .unwrap();

    let order = graph.topological_sort().unwrap();
    let pos_a = order.iter().position(|n| n == "A").unwrap();
    let pos_ab = order.iter().position(|n| n == "A.B").unwrap();
    let pos_abc = order.iter().position(|n| n == "A.B.C").unwrap();
    assert!(pos_a < pos_ab);
    assert!(pos_ab < pos_abc);
}

#[test]
fn test_nested_module_cyclic() {
    let mut graph = ModuleGraph::new();
    graph
        .add_module("A.B".to_string(), vec!["A.C".to_string()], None)
        .unwrap();
    graph
        .add_module("A.C".to_string(), vec!["A.B".to_string()], None)
        .unwrap();

    assert!(graph.detect_cycles().is_some());
}
