use super::*;

#[test]
fn test_knowledge_serialization() {
    let knowledge = Knowledge {
        project: ProjectInfo {
            name: "test-project".to_string(),
            version: "0.1.0".to_string(),
        },
        functions: vec![FunctionInfo {
            name: "add".to_string(),
            params: vec![
                ParamInfo {
                    name: "x".to_string(),
                    ty: "Int".to_string(),
                    description: Some("left operand".to_string()),
                },
                ParamInfo {
                    name: "y".to_string(),
                    ty: "Int".to_string(),
                    description: Some("right operand".to_string()),
                },
            ],
            return_type: "Int".to_string(),
            doc: Some("Add two integers".to_string()),
            module: Some("Math".to_string()),
            is_private: false,
        }],
        types: vec![TypeInfo {
            name: "Point".to_string(),
            kind: TypeKind::Record {
                fields: vec![
                    FieldInfo {
                        name: "x".to_string(),
                        ty: "Float".to_string(),
                    },
                    FieldInfo {
                        name: "y".to_string(),
                        ty: "Float".to_string(),
                    },
                ],
            },
            type_params: vec![],
            doc: Some("A 2D point".to_string()),
        }],
        dependencies: vec![DependencyInfo {
            from: "Main".to_string(),
            to: "Math".to_string(),
            kind: DependencyKind::Import,
        }],
    };

    let json = knowledge.to_json().unwrap();
    assert!(json.contains("test-project"));
    assert!(json.contains("add"));
    assert!(json.contains("Point"));
    assert!(json.contains("Main"));
}

#[test]
fn test_type_kind_variants() {
    let adt = TypeKind::Adt {
        variants: vec![
            VariantInfo {
                name: "Some".to_string(),
                fields: vec!["a".to_string()],
            },
            VariantInfo {
                name: "None".to_string(),
                fields: vec![],
            },
        ],
    };
    let json = serde_json::to_string(&adt).unwrap();
    assert!(json.contains("Some"));
    assert!(json.contains("None"));
}

#[test]
fn test_constrained_type() {
    let constrained = TypeKind::Constrained {
        base: "Int".to_string(),
        constraints: vec![">= 0".to_string(), "<= 100".to_string()],
    };
    let json = serde_json::to_string(&constrained).unwrap();
    assert!(json.contains(">= 0"));
}
