use super::*;

#[test]
fn test_link_empty_modules() {
    let modules: Vec<Module> = vec![];
    let linked = link_modules(&modules);
    assert!(linked.functions.is_empty());
    assert!(linked.gc_types.is_empty());
}

#[test]
fn test_link_single_module() {
    let module = Module {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![Instruction::I64Const(42)],
            is_export: true,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let linked = link_modules(&[module]);
    assert_eq!(linked.functions.len(), 1);
    assert_eq!(linked.functions[0].name, "main");
}

#[test]
fn test_link_two_modules() {
    let mod_a = Module {
        functions: vec![Function {
            name: "helper".to_string(),
            params: vec![IrType::I64],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::LocalGet(0),
                Instruction::I64Const(1),
                Instruction::I64Add,
            ],
            is_export: false,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let mod_b = Module {
        functions: vec![Function {
            name: "main".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I64Const(41),
                Instruction::Call(0), // mod_b 内の index 0 = helper(mod_a)
            ],
            is_export: true,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let linked = link_modules(&[mod_a, mod_b]);
    assert_eq!(linked.functions.len(), 2);
    assert_eq!(linked.functions[0].name, "helper");
    assert_eq!(linked.functions[1].name, "main");
}

#[test]
fn test_link_gc_type_rebase() {
    let mod_a = Module {
        functions: vec![],
        gc_types: vec![GcTypeDef {
            name: "Point".to_string(),
            kind: GcTypeKind::Struct(vec![
                GcField {
                    name: "x".to_string(),
                    ty: IrType::I64,
                    mutable: false,
                },
                GcField {
                    name: "y".to_string(),
                    ty: IrType::I64,
                    mutable: false,
                },
            ]),
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let mod_b = Module {
        functions: vec![Function {
            name: "make_point".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::I64Const(1),
                Instruction::I64Const(2),
                Instruction::StructNew(0), // mod_b 内の GC type 0
            ],
            is_export: true,
        }],
        gc_types: vec![GcTypeDef {
            name: "Color".to_string(),
            kind: GcTypeKind::Struct(vec![GcField {
                name: "r".to_string(),
                ty: IrType::I64,
                mutable: false,
            }]),
        }],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let linked = link_modules(&[mod_a, mod_b]);
    assert_eq!(linked.gc_types.len(), 2);
    assert_eq!(linked.gc_types[0].name, "Point");
    assert_eq!(linked.gc_types[1].name, "Color");

    // mod_b の StructNew(0) は新しいインデックス 1 にリベースされる
    if let Instruction::StructNew(idx) = &linked.functions[0].body[2] {
        assert_eq!(*idx, 1);
    } else {
        panic!("Expected StructNew");
    }
}

#[test]
fn test_link_funcref_rebases_function_and_type_indices() {
    let module_a = Module {
        functions: vec![Function {
            name: "a".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::RefFunc(0),
                Instruction::Drop,
                Instruction::RefFunc(1),
                Instruction::Drop,
                Instruction::CallRef(1),
                Instruction::Drop,
                Instruction::CallRef(2),
            ],
            is_export: false,
        }],
        gc_types: vec![GcTypeDef {
            name: "A".to_string(),
            kind: GcTypeKind::Struct(vec![]),
        }],
        imports: vec![ImportFunc {
            module: "env".to_string(),
            name: "a-import".to_string(),
            params: vec![],
            result: IrType::I64,
        }],
        globals: vec![],
        string_data: vec![],
    };
    let module_b = Module {
        functions: vec![Function {
            name: "b".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![],
            body: vec![
                Instruction::RefFunc(0),
                Instruction::Drop,
                Instruction::RefFunc(1),
                Instruction::Drop,
                Instruction::CallRef(1),
                Instruction::Drop,
                Instruction::CallRef(2),
            ],
            is_export: false,
        }],
        gc_types: vec![GcTypeDef {
            name: "B".to_string(),
            kind: GcTypeKind::Struct(vec![]),
        }],
        imports: vec![ImportFunc {
            module: "env".to_string(),
            name: "b-import".to_string(),
            params: vec![],
            result: IrType::I64,
        }],
        globals: vec![],
        string_data: vec![],
    };

    let linked = link_modules(&[module_a, module_b]);
    assert_eq!(linked.imports.len(), 2);
    assert_eq!(linked.functions.len(), 2);

    assert_ref_func(&linked.functions[0].body[0], 0);
    assert_ref_func(&linked.functions[0].body[2], 2);
    assert_call_ref(&linked.functions[0].body[4], 2);
    assert_call_ref(&linked.functions[0].body[6], 4);
    assert_ref_func(&linked.functions[1].body[0], 1);
    assert_ref_func(&linked.functions[1].body[2], 3);
    assert_call_ref(&linked.functions[1].body[4], 3);
    assert_call_ref(&linked.functions[1].body[6], 5);

    fn assert_ref_func(instruction: &Instruction, expected: u32) {
        match instruction {
            Instruction::RefFunc(index) => assert_eq!(*index, expected),
            other => panic!("expected RefFunc, got {other:?}"),
        }
    }

    fn assert_call_ref(instruction: &Instruction, expected: u32) {
        match instruction {
            Instruction::CallRef(index) => assert_eq!(*index, expected),
            other => panic!("expected CallRef, got {other:?}"),
        }
    }
}

#[test]
fn test_link_funcref_rebases_typed_local_and_gc_field_types() {
    let module_a = Module {
        functions: vec![Function {
            name: "a".to_string(),
            params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
            result: IrType::TypedFuncRef(2),
            locals: vec![IrType::Ref(0), IrType::TypedFuncRef(1)],
            body: vec![
                Instruction::RefFunc(0),
                Instruction::RefFunc(1),
                Instruction::CallRef(2),
            ],
            is_export: false,
        }],
        gc_types: vec![GcTypeDef {
            name: "A".to_string(),
            kind: GcTypeKind::Struct(vec![
                GcField {
                    name: "value".to_string(),
                    ty: IrType::Ref(0),
                    mutable: false,
                },
                GcField {
                    name: "call".to_string(),
                    ty: IrType::TypedFuncRef(2),
                    mutable: false,
                },
            ]),
        }],
        imports: vec![ImportFunc {
            module: "env".to_string(),
            name: "a-import".to_string(),
            params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
            result: IrType::Ref(0),
        }],
        globals: vec![],
        string_data: vec![],
    };
    let module_b = Module {
        functions: vec![Function {
            name: "b".to_string(),
            params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
            result: IrType::TypedFuncRef(2),
            locals: vec![IrType::Ref(0), IrType::TypedFuncRef(1)],
            body: vec![
                Instruction::RefFunc(0),
                Instruction::RefFunc(1),
                Instruction::CallRef(2),
            ],
            is_export: false,
        }],
        gc_types: vec![GcTypeDef {
            name: "B".to_string(),
            kind: GcTypeKind::Struct(vec![
                GcField {
                    name: "value".to_string(),
                    ty: IrType::Ref(0),
                    mutable: false,
                },
                GcField {
                    name: "call".to_string(),
                    ty: IrType::TypedFuncRef(2),
                    mutable: false,
                },
            ]),
        }],
        imports: vec![ImportFunc {
            module: "env".to_string(),
            name: "b-import".to_string(),
            params: vec![IrType::Ref(0), IrType::TypedFuncRef(2)],
            result: IrType::Ref(0),
        }],
        globals: vec![],
        string_data: vec![],
    };

    let linked = link_modules(&[module_a, module_b]);
    assert_eq!(
        linked.imports[0].params,
        vec![IrType::Ref(0), IrType::TypedFuncRef(4)]
    );
    assert_eq!(linked.imports[0].result, IrType::Ref(0));
    assert_eq!(
        linked.imports[1].params,
        vec![IrType::Ref(1), IrType::TypedFuncRef(5)]
    );
    assert_eq!(linked.imports[1].result, IrType::Ref(1));
    assert_eq!(
        linked.functions[0].params,
        vec![IrType::Ref(0), IrType::TypedFuncRef(4)]
    );
    assert_eq!(linked.functions[0].result, IrType::TypedFuncRef(4));
    assert_eq!(
        linked.functions[0].locals,
        vec![IrType::Ref(0), IrType::TypedFuncRef(2)]
    );
    assert_eq!(
        linked.functions[1].params,
        vec![IrType::Ref(1), IrType::TypedFuncRef(5)]
    );
    assert_eq!(linked.functions[1].result, IrType::TypedFuncRef(5));
    assert_eq!(
        linked.functions[1].locals,
        vec![IrType::Ref(1), IrType::TypedFuncRef(3)]
    );

    fn assert_gc_field_types(gc_type: &GcTypeDef, expected_ref: u32, expected_func: u32) {
        let GcTypeKind::Struct(fields) = &gc_type.kind else {
            panic!("expected struct GC type: {gc_type:?}");
        };
        assert_eq!(fields[0].ty, IrType::Ref(expected_ref));
        assert_eq!(fields[1].ty, IrType::TypedFuncRef(expected_func));
    }

    assert_gc_field_types(&linked.gc_types[0], 0, 4);
    assert_gc_field_types(&linked.gc_types[1], 1, 5);
}

#[test]
fn test_link_funcref_rebases_array_element_type() {
    fn module(name: &str) -> Module {
        Module {
            functions: vec![Function {
                name: name.to_string(),
                params: vec![],
                result: IrType::I64,
                locals: vec![],
                body: vec![Instruction::I64Const(0)],
                is_export: false,
            }],
            gc_types: vec![GcTypeDef {
                name: format!("{name}-array"),
                kind: GcTypeKind::Array(IrType::TypedFuncRef(1)),
            }],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        }
    }

    let linked = link_modules(&[module("left"), module("right")]);
    assert_eq!(linked.gc_types.len(), 2);
    assert_eq!(linked.functions.len(), 2);

    assert_array_element_type(&linked.gc_types[0], IrType::TypedFuncRef(2));
    assert_array_element_type(&linked.gc_types[1], IrType::TypedFuncRef(3));

    fn assert_array_element_type(gc_type: &GcTypeDef, expected: IrType) {
        let GcTypeKind::Array(element_type) = &gc_type.kind else {
            panic!("expected array GC type: {gc_type:?}");
        };
        assert_eq!(*element_type, expected);
    }
}
