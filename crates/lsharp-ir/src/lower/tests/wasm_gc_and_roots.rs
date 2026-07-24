//! WasmGC と root lifetime の lowering 契約

use super::*;

#[test]
fn wasm_gc_lowering_registers_string_bytes_as_packed_array() {
    let program = lsharp_syntax::parse(r#"(defn main [] (string-length "hello"))"#).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .unwrap();

    assert!(matches!(
        module.gc_types.last().map(|gc_type| &gc_type.kind),
        Some(GcTypeKind::PackedByteArray)
    ));
}

#[test]
fn wasm_gc_closure_lowering_rejects_linear_memory_fallback_explicitly() {
    let program = lsharp_syntax::parse(
        r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] 0)
        "#,
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let error = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect_err("WasmGC closure は linear-memory fallback を生成してはならない");

    assert!(matches!(error, LowerError::Unsupported { .. }));
    assert!(error.to_string().contains("typed funcref/env struct"));
}

#[test]
fn wasm_gc_captured_lambda_direct_call_lowers_to_env_struct_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [n] ((fn [x] (+ x n)) 41))
        "#,
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("captured lambda direct call は env struct + call_ref へ lowering できる");

    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main が存在する");
    assert!(
        main.body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::StructNew(_))),
        "captured lambda は env struct を生成するべき: {:?}",
        main.body
    );
    assert!(
        main.body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::CallRef(_))),
        "captured lambda direct call は typed call_ref を生成するべき: {:?}",
        main.body
    );
    assert!(
        main.body.iter().all(|instruction| !matches!(
            instruction,
            Instruction::CallIndirect(_)
                | Instruction::FuncIdx(_)
                | Instruction::I64Load { .. }
                | Instruction::I64Store { .. }
        )),
        "captured lambda は linear-memory closure fallback を生成しない: {:?}",
        main.body
    );
}

#[test]
fn wasm_gc_captured_lambda_let_alias_lowers_to_env_struct_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [n]
          (let [f (fn [x] (+ x n))]
            (f 41)))
        "#,
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("captured lambda let alias は env struct + call_ref へ lowering できる");

    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main が存在する");
    assert!(
        main.body.windows(2).any(|instructions| {
            matches!(
                instructions,
                [Instruction::LocalGet(_), Instruction::StructGet(_, 0)]
            )
        }),
        "captured env alias は local から function field を取得するべき: {:?}",
        main.body
    );
    assert!(
        main.body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::CallRef(_))),
        "captured lambda let alias は typed call_ref を生成するべき: {:?}",
        main.body
    );
}

#[test]
fn wasm_gc_non_capturing_lambda_lowers_to_funcref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn main [] 0)
        "#,
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("captured なし lambda は WasmGC funcref slice へ lowering できる");

    let make_inc = module
        .functions
        .iter()
        .find(|function| function.name == "make-inc")
        .expect("make-inc が存在する");
    assert_eq!(make_inc.result, IrType::FuncRef);
    assert!(
        make_inc
            .body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::RefFunc(_))),
        "non-capturing lambda は ref.func を生成するべき: {:?}",
        make_inc.body
    );
    assert!(make_inc.body.iter().all(|instruction| {
        !matches!(
            instruction,
            Instruction::Call(1)
                | Instruction::FuncIdx(_)
                | Instruction::I32Store { .. }
                | Instruction::I64Store { .. }
        )
    }));

    let lifted = module
        .functions
        .iter()
        .find(|function| function.name.starts_with("__lambda"))
        .expect("lifted lambda が存在する");
    assert_eq!(lifted.params, vec![IrType::I64]);
    assert!(
        lifted
            .body
            .iter()
            .all(|instruction| !matches!(instruction, Instruction::I64Load { .. }))
    );
}

#[test]
fn wasm_gc_non_capturing_lambda_call_lowers_to_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main [] ((fn [x] (+ x 1)) 41))
        "#,
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("non-capturing lambda call は WasmGC call_ref へ lowering できる");

    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main が存在する");
    assert!(
        main.body.windows(2).any(|instructions| {
            matches!(
                instructions,
                [Instruction::RefFunc(1), Instruction::CallRef(2)]
            )
        }),
        "main body: {:?}",
        main.body
    );
}

#[test]
fn wasm_gc_local_non_capturing_lambda_call_lowers_to_call_ref() {
    let program = lsharp_syntax::parse(
        r#"
        (defn main []
          (let [f (fn [x] (+ x 1))]
            (f 41)))
        "#,
    )
    .unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .expect("local non-capturing lambda call は WasmGC call_ref へ lowering できる");

    let main = module
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main が存在する");
    assert!(
        main.body.windows(2).any(|instructions| {
            matches!(
                instructions,
                [Instruction::LocalGet(_), Instruction::CallRef(_)]
            )
        }),
        "local alias は concrete typed funcref local をそのまま call_ref へ渡すべき: main: {:?}, locals: {:?}",
        main.body,
        main.locals
    );
    assert!(
        main.body.windows(2).all(|instructions| {
            !matches!(
                instructions,
                [Instruction::RefFunc(_), Instruction::CallRef(_)]
            )
        }),
        "local alias の call site は ref.func を再 materialize しない: {:?}",
        main.body
    );
}

#[test]
fn test_root_lifetime_ledger_rejects_stale_slot_after_pop() {
    let function = Function {
        name: "stale-root".to_string(),
        params: Vec::new(),
        result: crate::IrType::I64,
        locals: vec![crate::IrType::I64],
        body: vec![
            Instruction::I64Const(42),
            Instruction::Call(ROOT_PUSH_IDX),
            Instruction::LocalSet(0),
            Instruction::Call(ROOT_POP_IDX),
            Instruction::LocalGet(0),
            Instruction::I64Const(7),
            Instruction::Call(ROOT_SET_IDX),
        ],
        is_export: false,
    };

    let error = validate_function(&function).expect_err("pop 済み slot の root_set は拒否すべき");
    assert!(
        matches!(error, RootLifetimeError::StaleSlot { .. }),
        "stale slot を専用エラーとして報告すべき: {error:?}"
    );
}

#[test]
fn test_root_lifetime_ledger_accepts_root_set_before_pop() {
    let function = Function {
        name: "valid-root".to_string(),
        params: Vec::new(),
        result: IrType::I64,
        locals: vec![IrType::I64],
        body: vec![
            Instruction::I64Const(42),
            Instruction::Call(ROOT_PUSH_IDX),
            Instruction::LocalSet(0),
            Instruction::LocalGet(0),
            Instruction::I64Const(7),
            Instruction::Call(ROOT_SET_IDX),
            Instruction::Drop,
            Instruction::Call(ROOT_POP_IDX),
            Instruction::Drop,
        ],
        is_export: false,
    };

    validate_function(&function).expect("active slot への root_set は root_pop 前なら有効");
}

#[test]
fn test_root_lifetime_ledger_rejects_branch_depth_mismatch() {
    let function = Function {
        name: "branch-root".to_string(),
        params: Vec::new(),
        result: IrType::I64,
        locals: Vec::new(),
        body: vec![
            Instruction::IfEmpty,
            Instruction::I64Const(42),
            Instruction::Call(ROOT_PUSH_IDX),
            Instruction::Drop,
            Instruction::Else,
            Instruction::End,
        ],
        is_export: false,
    };

    let error =
        validate_function(&function).expect_err("分岐間で root depth がずれる IR は拒否すべき");
    assert!(
        matches!(error, RootLifetimeError::BranchDepthMismatch { .. }),
        "branch depth mismatch を専用エラーとして報告すべき: {error:?}"
    );
}

#[test]
fn test_root_lifetime_ledger_accepts_lowered_allocating_fixtures() {
    for source in [
        r#"(defn main [] (string-concat "a" "b"))"#,
        r#"(defn main [] (vector-push (vector-new 0) "x"))"#,
        r#"(defn main [] (map-insert (map-new) (vector-new 0) "value"))"#,
    ] {
        let module = lower(source);
        validate_module(&module).unwrap_or_else(|error| {
            panic!("lowered root lifetime が壊れている: source={source:?}, error={error:?}")
        });
    }
}

#[test]
fn lower_errors_expose_stable_codes_and_spans() {
    let span = Span::new(5, 12);
    let errors = vec![
        (
            LowerError::Unsupported {
                msg: "unsupported".to_string(),
                span: Some(span),
            },
            "LS3001",
        ),
        (
            LowerError::UndefinedFunction {
                name: "missing".to_string(),
                span: Some(span),
            },
            "LS3002",
        ),
    ];

    for (error, expected_code) in errors {
        assert_eq!(error.code(), expected_code);
        assert_eq!(error.span(), Some(span));
    }

    let root_error = LowerError::RootLifetime {
        error: RootLifetimeError::RootPopUnderflow {
            function: "main".to_string(),
            instruction_index: 3,
        },
    };
    assert_eq!(root_error.code(), "LS3003");
    assert_eq!(root_error.span(), None);
}
