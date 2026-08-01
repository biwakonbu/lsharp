#[test]
fn wasm_gc_emitter_executes_lowered_non_capturing_lambda_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [] ((fn [x] (+ x 1)) 41))
        "#,
    )
    .expect("non-capturing lambda call source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("non-capturing lambda call source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("non-capturing lambda call を WasmGC call_ref IR へ lowering できる");

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("lowered non-capturing lambda call module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("typed funcref を有効化した engine を作成できる");
    let module = Module::new(&engine, bytes).expect("lowered lambda call module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_offsets_lowered_lambda_call_ref_after_print_string_import() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [] (do (print "hello") ((fn [x] (+ x 1)) 41)))
        "#,
    )
    .expect("print と non-capturing lambda call source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("print と non-capturing lambda call source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("print と non-capturing lambda call を WasmGC IR へ lowering できる");

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("print import 付き lambda call module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("typed funcref を有効化した engine を作成できる");
    Module::new(&engine, bytes).expect("print import 付き lambda call module を検証できる");
}

#[test]
fn wasm_gc_emitter_executes_local_non_capturing_lambda_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main []
          (let [f (fn [x] (+ x 1))]
            (f 41)))
        "#,
    )
    .expect("local non-capturing lambda call source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("local non-capturing lambda call source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("local non-capturing lambda call を WasmGC IR へ lowering できる");

    let bytes =
        lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir).expect("local lambda call module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("typed funcref を有効化した engine を作成できる");
    let module = Module::new(&engine, bytes).expect("local lambda call module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_offsets_local_typed_funcref_after_print_string_import() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main []
          (do
            (print "hello")
            (let [f (fn [x] (+ x 1))]
              (f 41))))
        "#,
    )
    .expect("print と local non-capturing lambda call source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("print と local non-capturing lambda call source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("print と local non-capturing lambda call を WasmGC IR へ lowering できる");

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("print import 付き local lambda call module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("typed funcref を有効化した engine を作成できる");
    Module::new(&engine, bytes).expect("print import 付き typed local lambda module を検証できる");
}

#[test]
fn wasm_gc_emitter_executes_captured_lambda_env_struct_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [n] ((fn [x] (+ x n)) 41))
        "#,
    )
    .expect("captured lambda call source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("captured lambda call source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("captured lambda を WasmGC env struct IR へ lowering できる");
    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("captured lambda env struct module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("captured lambda module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<i64, i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, 1).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_validates_linked_typed_funcref_and_gc_types() {
    fn typed_function(name: &str, is_export: bool, result: i64) -> Function {
        Function {
            name: name.to_string(),
            params: vec![IrType::TypedFuncRef(1), IrType::Ref(0)],
            result: IrType::I64,
            locals: vec![IrType::TypedFuncRef(1), IrType::Ref(0)],
            body: vec![Instruction::I64Const(result)],
            is_export,
        }
    }

    fn module(name: &str, main_result: i64, main_export: bool) -> IrModule {
        IrModule {
            functions: vec![
                typed_function(name, false, 0),
                Function {
                    name: format!("{name}-main"),
                    params: vec![],
                    result: IrType::I64,
                    locals: vec![],
                    body: vec![Instruction::I64Const(main_result)],
                    is_export: main_export,
                },
            ],
            gc_types: vec![GcTypeDef {
                name: format!("{name}-env"),
                kind: GcTypeKind::Struct(vec![
                    GcField {
                        name: "capture".to_string(),
                        ty: IrType::Ref(0),
                        mutable: false,
                    },
                    GcField {
                        name: "call".to_string(),
                        ty: IrType::TypedFuncRef(1),
                        mutable: false,
                    },
                ]),
            }],
            imports: vec![],
            globals: vec![],
            string_data: vec![],
        }
    }

    let linked = link_modules(&[module("left", 0, false), module("right", 42, true)]);
    assert_eq!(linked.functions[0].params[0], IrType::TypedFuncRef(2));
    assert_eq!(linked.functions[1].params, Vec::<IrType>::new());
    assert_eq!(linked.functions[2].params[0], IrType::TypedFuncRef(4));
    let GcTypeKind::Struct(left_fields) = &linked.gc_types[0].kind else {
        panic!("linked left env must remain a struct");
    };
    assert_eq!(left_fields[0].ty, IrType::Ref(0));
    assert_eq!(left_fields[1].ty, IrType::TypedFuncRef(2));
    let GcTypeKind::Struct(right_fields) = &linked.gc_types[1].kind else {
        panic!("linked right env must remain a struct");
    };
    assert_eq!(right_fields[0].ty, IrType::Ref(1));
    assert_eq!(right_fields[1].ty, IrType::TypedFuncRef(4));

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&linked)
        .expect("linked typed funcref/GC type module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module =
        Module::new(&engine, bytes).expect("linked typed funcref/GC type module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("linked module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "right-main")
        .expect("linked right-main export が存在する");
    assert_eq!(main.call(&mut store, ()).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_offsets_captured_env_funcref_after_print_string_import() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [n]
          (do
            (print "hello")
            ((fn [x] (+ x n)) 41)))
        "#,
    )
    .expect("print と captured lambda call source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("print と captured lambda call source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("print と captured lambda を WasmGC env struct IR へ lowering できる");

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("print import 付き captured lambda module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    Module::new(&engine, bytes).expect("print import 付き captured lambda module を検証できる");
}

#[test]
fn wasm_gc_emitter_executes_captured_lambda_let_alias_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [n]
          (let [f (fn [x] (+ x n))]
            (f 41)))
        "#,
    )
    .expect("captured lambda let alias source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("captured lambda let alias source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("captured lambda let alias を WasmGC env struct IR へ lowering できる");

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("captured lambda let alias module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    config.wasm_reference_types(true);
    config.wasm_function_references(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module =
        Module::new(&engine, bytes).expect("captured lambda let alias module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<i64, i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, 1).unwrap(), 42);
}

#[test]
fn wasm_gc_emitter_executes_lowered_nested_parametric_record_pattern() {
    let program = lsharp_syntax::parse(
        r#"
        (type (Box a) (record (: value a)))
        (type (Outer a) (record (: inner (Box a))))
        (defn read-inner [o]
          (match o
            [{Outer inner {Box value x}} x]
            [_ 0]))
        (defn main [] (read-inner {Outer inner {Box value 41}}))
        "#,
    )
    .expect("nested parametric record pattern source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("nested parametric record pattern source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("nested parametric record pattern を WasmGC IR へ lowering できる");

    let bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir)
        .expect("nested parametric record pattern module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("nested record module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[]).expect("module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 41);
}

#[test]
fn wasm_gc_emitter_executes_lowered_adt_pattern_with_typed_payload() {
    let program = lsharp_syntax::parse(
        r#"
        (type Option (Some Int) None)
        (defn from-option [value]
          (match value
            [(Some x) x]
            [None 0]))
        (defn main []
          (+
            (from-option (Some 41))
            (if (= (from-option None) 0) 1 0)))
        "#,
    )
    .expect("WasmGC ADT pattern source を parse できる");
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .expect("WasmGC ADT pattern source を型推論できる");
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = lsharp_ir::lower::Lower::with_backend(lsharp_ir::lower::LowerBackend::WasmGc);
    let ir = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("WasmGC ADT pattern を IR へ lowering できる");
    let bytes =
        lsharp_wasm::wasmgc::emit_wasm_wasmgc(&ir).expect("WasmGC ADT pattern module を生成できる");
    let mut config = Config::new();
    config.wasm_gc(true);
    let engine = Engine::new(&config).expect("WasmGC engine を作成できる");
    let module = Module::new(&engine, bytes).expect("WasmGC ADT module を検証できる");
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).expect("WasmGC ADT module を instantiate できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .expect("main export が存在する");

    assert_eq!(main.call(&mut store, ()).unwrap(), 42);
}
