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

    let error = validate_function(&function, &RootLifetimeExemptions::default())
        .expect_err("pop 済み slot の root_set は拒否すべき");
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

    validate_function(&function, &RootLifetimeExemptions::default())
        .expect("active slot への root_set は root_pop 前なら有効");
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

    let error = validate_function(&function, &RootLifetimeExemptions::default())
        .expect_err("分岐間で root depth がずれる IR は拒否すべき");
    assert!(
        matches!(error, RootLifetimeError::BranchDepthMismatch { .. }),
        "branch depth mismatch を専用エラーとして報告すべき: {error:?}"
    );
}

#[test]
fn test_root_lifetime_ledger_accepts_explicit_cross_function_root_lease_helpers() {
    let acquire = Function {
        name: "typeinfer-builtin-root-value".to_string(),
        params: vec![IrType::I64],
        result: IrType::I64,
        locals: Vec::new(),
        body: vec![
            Instruction::LocalGet(0),
            Instruction::Call(ROOT_PUSH_IDX),
            Instruction::Drop,
            Instruction::LocalGet(0),
        ],
        is_export: false,
    };
    let release = Function {
        name: "typeinfer-builtin-release-roots".to_string(),
        params: vec![IrType::I64],
        result: IrType::I64,
        locals: Vec::new(),
        body: vec![
            Instruction::LocalGet(0),
            Instruction::IfEmpty,
            Instruction::I64Const(0),
            Instruction::Else,
            Instruction::Call(ROOT_POP_IDX),
            Instruction::Drop,
            Instruction::I64Const(0),
            Instruction::End,
        ],
        is_export: false,
    };

    validate_function(&acquire, &RootLifetimeExemptions::default())
        .expect("builtin root acquire helper は lease を返せるべき");
    validate_function(&release, &RootLifetimeExemptions::default())
        .expect("builtin root release helper は caller の lease を解放できるべき");
}

#[test]
fn test_root_lifetime_ledger_rejects_unannotated_cross_function_root_lease() {
    let function = Function {
        name: "unannotated-root-lease".to_string(),
        params: Vec::new(),
        result: IrType::I64,
        locals: Vec::new(),
        body: vec![
            Instruction::I64Const(42),
            Instruction::Call(ROOT_PUSH_IDX),
            Instruction::Drop,
        ],
        is_export: false,
    };

    let error = validate_function(&function, &RootLifetimeExemptions::default())
        .expect_err("未登録 helper の root lease は拒否すべき");
    assert!(
        matches!(error, RootLifetimeError::ImbalancedExit { depth: 1, .. }),
        "通常関数の root lease は ImbalancedExit として拒否すべき: {error:?}"
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
        validate_module(&module, &RootLifetimeExemptions::default()).unwrap_or_else(|error| {
            panic!("lowered root lifetime が壊れている: source={source:?}, error={error:?}")
        });
    }
}

/// lowering の結果を `Result` のまま返す。
/// root lifetime 検証は lowering 内部 (`program.rs:223`) で走るため、共有の `lower` helper は
/// 失敗時に panic する。verifier 自体の挙動を検査するテストではこちらを使う。
fn try_lower(source: &str) -> Result<Module, LowerError> {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::new();
    lowerer.lower_program_with_expr_types(&program, &type_results, &expr_type_results)
}

#[test]
fn test_root_lifetime_ledger_accepts_main_holding_root_at_exit() {
    // WASI entry の出口に残る slot は、直後にプログラムが終了するので stale になり得ない。
    // root_push / root_pop は runtime spec の公開 API で、均衡は要求されていない。
    let module = try_lower(
        r#"
        (defn main []
          (let [keep (string-concat "keep" "!")
                _slot (root_push keep)]
            0))
        "#,
    )
    .expect("main が root を保持したまま終わる module は lowering を通すべき");

    validate_module(&module, &RootLifetimeExemptions::default())
        .expect("module 単体の再検証も通るべき");
}

#[test]
fn test_root_lifetime_ledger_still_rejects_lowered_non_main_imbalance() {
    // 免除が main の外へ広がっていないことの保証。
    let error = try_lower(
        r#"
        (defn hold [x] (do (root_push x) 0))
        (defn main [] (hold 1))
        "#,
    )
    .expect_err("非 main 関数の不均衡な root lease は拒否し続けるべき");

    assert!(
        matches!(
            error,
            LowerError::RootLifetime {
                error: RootLifetimeError::ImbalancedExit { ref function, .. }
            } if function == "hold"
        ),
        "免除は main だけに閉じているべき: {error:?}"
    );
    assert_eq!(
        error.code(),
        "LS3003",
        "診断コードは LS3003 のままであるべき"
    );
}

#[test]
fn test_root_lifetime_ledger_rejects_non_exported_function_named_main() {
    // 免除条件は名前だけではなく `is_export` との連言である。
    let function = Function {
        name: "main".to_string(),
        params: Vec::new(),
        result: IrType::I64,
        locals: Vec::new(),
        body: vec![
            Instruction::I64Const(42),
            Instruction::Call(ROOT_PUSH_IDX),
            Instruction::Drop,
        ],
        is_export: false,
    };

    let error = validate_function(&function, &RootLifetimeExemptions::default())
        .expect_err("export されない main は WASI entry ではないので免除しない");
    assert!(
        matches!(error, RootLifetimeError::ImbalancedExit { depth: 1, .. }),
        "免除条件は is_export と名前の両方であるべき: {error:?}"
    );
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

#[test]
fn test_root_lifetime_ledger_accepts_annotated_intentional_imbalance() {
    // :roots-unbalanced が付いた関数は抽象実行ごと省略する (lease release helper の先例と同形)。
    let module = try_lower(
        r#"
        (defn hold [x]
          :roots-unbalanced "呼び出し側が解放する root lease"
          (do (root_push x) 0))
        (defn main [] (hold 1))
        "#,
    )
    .expect("注釈された不均衡は lowering を通すべき");

    let exemptions = RootLifetimeExemptions::from_names(["hold".to_string()]);
    validate_module(&module, &exemptions).expect("免除集合を渡せば module 単体でも通るべき");

    // 免除は IR に載らず lowering 時点の AST から導かれる。この非対称性は ADR に記録済み。
    validate_module(&module, &RootLifetimeExemptions::default())
        .expect_err("空の免除集合では従来どおり拒否されるべき");
}

#[test]
fn test_root_lifetime_ledger_still_rejects_same_shape_without_annotation() {
    // 上のテストと注釈 1 行だけが違う対。緩和が注釈に紐づいていることの保証。
    let error = try_lower(
        r#"
        (defn hold [x] (do (root_push x) 0))
        (defn main [] (hold 1))
        "#,
    )
    .expect_err("注釈の無い不均衡は拒否し続けるべき");

    assert!(
        matches!(
            error,
            LowerError::RootLifetime {
                error: RootLifetimeError::ImbalancedExit { ref function, .. }
            } if function == "hold"
        ),
        "注釈が無ければ従来どおり ImbalancedExit: {error:?}"
    );
}

#[test]
fn test_root_lifetime_annotation_does_not_leak_to_sibling_function() {
    // 同一 module の別関数へ免除が漏れないこと。
    let error = try_lower(
        r#"
        (defn hold-a [x]
          :roots-unbalanced "意図的に保持する"
          (do (root_push x) 0))
        (defn hold-b [x] (do (root_push x) 0))
        (defn main [] (+ (hold-a 1) (hold-b 2)))
        "#,
    )
    .expect_err("注釈の無い hold-b は拒否されるべき");

    assert!(
        matches!(
            error,
            LowerError::RootLifetime {
                error: RootLifetimeError::ImbalancedExit { ref function, .. }
            } if function == "hold-b"
        ),
        "免除は注釈された関数だけに閉じているべき: {error:?}"
    );
}

#[test]
fn test_root_lifetime_ledger_accepts_annotated_underflowing_pop() {
    // 空 stack への root_pop は runtime spec 上 0 を返す合法な呼び出しだが、既定では拒否する。
    try_lower(
        r#"
        (defn main []
          :roots-unbalanced "空 root stack への pop が 0 を返すことを確認する fixture"
          (do (root_pop) 0))
        "#,
    )
    .expect("注釈された underflow pop は lowering を通すべき");
}

#[test]
fn test_root_lifetime_ledger_accepts_annotated_branch_depth_mismatch() {
    // 実 fixture (push-roots) と同形。if の then / else で root 深さが揃わない。
    try_lower(
        r#"
        (defn push-roots [n]
          :roots-unbalanced "root stack の grow を確認するため意図的に積み増す"
          (if (<= n 0)
            0
            (do (root_push n) (push-roots (- n 1)))))
        (defn main [] (push-roots 3))
        "#,
    )
    .expect("注釈された branch depth mismatch は lowering を通すべき");
}
