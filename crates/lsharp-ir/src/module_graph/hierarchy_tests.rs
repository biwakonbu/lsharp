use super::*;

#[test]
fn test_parent_module() {
    assert_eq!(ModuleGraph::parent_module("A.B.C"), Some("A.B"));
    assert_eq!(ModuleGraph::parent_module("A.B"), Some("A"));
    assert_eq!(ModuleGraph::parent_module("A"), None);
}

#[test]
fn test_children() {
    let mut graph = ModuleGraph::new();
    graph.add_module("App".to_string(), vec![], None).unwrap();
    graph
        .add_module("App.Utils".to_string(), vec![], None)
        .unwrap();
    graph
        .add_module("App.Models".to_string(), vec![], None)
        .unwrap();
    graph
        .add_module("App.Models.User".to_string(), vec![], None)
        .unwrap();

    let mut children = graph.children("App");
    children.sort();
    assert_eq!(children, vec!["App.Models", "App.Utils"]);

    let children_models = graph.children("App.Models");
    assert_eq!(children_models, vec!["App.Models.User"]);
}

#[test]
fn test_descendants() {
    let mut graph = ModuleGraph::new();
    graph.add_module("App".to_string(), vec![], None).unwrap();
    graph
        .add_module("App.Utils".to_string(), vec![], None)
        .unwrap();
    graph
        .add_module("App.Models".to_string(), vec![], None)
        .unwrap();
    graph
        .add_module("App.Models.User".to_string(), vec![], None)
        .unwrap();

    let mut desc = graph.descendants("App");
    desc.sort();
    assert_eq!(desc, vec!["App.Models", "App.Models.User", "App.Utils"]);
}
