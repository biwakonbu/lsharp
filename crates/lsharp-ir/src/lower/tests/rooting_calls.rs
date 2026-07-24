//! allocating call site の GC root 回帰

use super::*;

#[test]
fn test_lower_string_concat_auto_roots_arguments() {
    let module = lower(r#"(defn main [] (string-concat "a" "b"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    let concat_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Call(2)))
        .expect("string-concat は __string_concat を呼ぶべき");
    let pushes_before_concat = main_fn.body[..concat_pos]
        .iter()
        .filter(|instr| matches!(instr, Instruction::Call(14)))
        .count();
    let pops_after_concat = main_fn.body[concat_pos + 1..]
        .iter()
        .filter(|instr| matches!(instr, Instruction::Call(15)))
        .count();

    assert!(
        pushes_before_concat >= 2,
        "string-concat 前に 2 つの root_push が必要: {:?}",
        main_fn.body
    );
    assert!(
        pops_after_concat >= 2,
        "string-concat 後に 2 つの root_pop が必要: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_substring_auto_roots_source_string() {
    let module = lower(r#"(defn main [] (substring "abcd" 1 3))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        1,
        "substring は source string を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "substring は alloc 後に root_pop で解放すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_ref_new_auto_roots_wrapped_heap_value() {
    let module = lower(r#"(defn main [] (ref-new "ab"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "ref-new は __alloc 前に wrapped value を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "ref-new は __alloc 後に root_pop で解放すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_vector_push_auto_roots_realloc_inputs() {
    let module = lower(r#"(defn main [] (vector-push (vector-new 0) "x"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 2,
        "vector-push の再割り当ては old vector / pushed value を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 2,
        "vector-push の再割り当ては alloc 後に root_pop で解放すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_vector_push_balances_root_push_and_pop() {
    let module = lower(r#"(defn main [] (vector-push (vector-new 0) "x"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        count_call_instr(&main_fn.body, 15),
        "vector-push は receiver/value の root_push と root_pop を釣り合わせるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_string_concat_roots_lhs_before_lowering_rhs() {
    let module = lower(r#"(defn main [] (string-concat "a" "b"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 2,
        "2 つの文字列リテラル割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        !root_push_positions.is_empty(),
        "string-concat は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "lhs は rhs の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_wasmgc_substring_rejects_static_invalid_range() {
    let program = lsharp_syntax::parse(r#"(defn main [] (substring "abc" 3 2))"#).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let result = lowerer.lower_program_with_expr_types(&program, &type_results, &expr_type_results);

    let error = result.expect_err("WasmGC の静的 invalid substring range は診断すべき");
    assert_eq!(error.code(), "LS3001");
    assert!(error.to_string().contains("substring"));
    assert!(error.span().is_some());
}

#[test]
fn test_wasmgc_substring_emits_dynamic_range_trap() {
    let source = r#"
        (defn slice [value start end] (substring value start end))
        (defn main [] (string-length (slice "abc" 1 2)))
    "#;
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let expr_type_results = infer.expr_type_results_snapshot();
    let mut lowerer = Lower::with_backend(LowerBackend::WasmGc);
    let module = lowerer
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .unwrap();
    let slice = module.functions.iter().find(|f| f.name == "slice").unwrap();

    assert!(
        slice
            .body
            .iter()
            .any(|instruction| matches!(instruction, Instruction::Unreachable)),
        "動的 substring range は invalid 値を trap する guard を含むべき: {:?}",
        slice.body
    );
}

#[test]
fn test_lower_substring_roots_source_before_lowering_index_exprs() {
    let module = lower(r#"(defn main [] (substring "abcd" (string-length "xy") 2))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 2,
        "source 文字列と index 側文字列の割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        !root_push_positions.is_empty(),
        "substring は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "source string は start/end 式の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_vector_push_roots_vector_before_lowering_value_expr() {
    let module = lower(r#"(defn main [] (vector-push (vector-new 0) "x"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 3,
        "vector-new / pushed string / realloc の 3 回割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions.len() >= 2,
        "vector-push は vector と value を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "receiver vector は value 式の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_insert_roots_map_before_lowering_key_expr() {
    let module = lower(r#"(defn main [] (map-insert (map-new) (string-concat "a" "b") 1))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let alloc_positions = call_positions(&main_fn.body, 1);
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert!(
        alloc_positions.len() >= 3,
        "map-new と key 側の文字列割り当てが必要: {:?}",
        main_fn.body
    );
    assert!(
        !root_push_positions.is_empty(),
        "map-insert は map receiver を root_push で保護すべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[0] > alloc_positions[0] && root_push_positions[0] < alloc_positions[1],
        "map receiver は key/value 式の割り当て前に root_push で保護すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_insert_roots_heap_value_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-insert (map-new) 1 "value"))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let loop_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "map-insert は loop backedge をまたぐ heap value も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "map-insert は receiver/value に対応する root_pop を 2 回使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < loop_pos,
        "map-insert の heap value は loop へ入る前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_insert_does_not_root_int_value_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-insert (map-new) 1 42))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        1,
        "map-insert の Int value は receiver 以外を root_push しないべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "map-insert の Int value は receiver 分だけ root_pop すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_insert_roots_heap_key_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-insert (map-new) (vector-new 0) 1))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let loop_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "map-insert は loop backedge をまたぐ heap key も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "map-insert は receiver/key に対応する root_pop を 2 回使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < loop_pos,
        "map-insert の heap key は loop へ入る前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_get_roots_heap_key_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-get (map-new) (vector-new 0)))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let loop_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "map-get は loop backedge をまたぐ heap key も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "map-get は receiver/key に対応する root_pop を 2 回使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < loop_pos,
        "map-get の heap key は loop へ入る前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_contains_roots_heap_key_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-contains? (map-new) (vector-new 0)))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let loop_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "map-contains? は loop backedge をまたぐ heap key も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "map-contains? は receiver/key に対応する root_pop を 2 回使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < loop_pos,
        "map-contains? の heap key は loop へ入る前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_remove_roots_heap_key_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-remove (map-new) (vector-new 0)))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);
    let loop_pos = main_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        2,
        "map-remove は loop backedge をまたぐ heap key も root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "map-remove は receiver/key に対応する root_pop を 2 回使うべき: {:?}",
        main_fn.body
    );
    assert!(
        root_push_positions[1] < loop_pos,
        "map-remove の heap key は loop へ入る前に root_push で保護されるべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_map_get_does_not_root_int_key_across_loop_backedge() {
    let module = lower(r#"(defn main [] (map-get (map-new) 1))"#);
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        1,
        "map-get の Int key は receiver 以外を root_push しないべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "map-get の Int key は receiver 分だけ root_pop すべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_string_argument() {
    let module = lower(
        r#"
        (defn consume-string [s] (string-length s))
        (defn main [] (consume-string "hello"))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "string 引数の user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "string 引数の user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_read_stdin_argument() {
    let module = lower(
        r#"
        (defn consume-string [s] (string-length s))
        (defn main [] (consume-string (read-stdin)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "read-stdin 由来の string 引数を使う user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "read-stdin 由来の string 引数を使う user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_generic_identity_result_argument() {
    let module = lower(
        r#"
        (defn id [x] x)
        (defn consume-string [s] (string-length s))
        (defn main [] (consume-string (id "hello")))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        2,
        "generic identity の戻り値を使う user call は inner call の literal 保護に加えて outer call 用にも root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        2,
        "generic identity の戻り値を使う user call は inner/outer 分の root_pop を行うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_local_generic_closure_result_argument() {
    let module = lower(
        r#"
        (defn consume-string [s] (string-length s))
        (defn main []
          (let [id (fn [x] x)]
            (consume-string (id "hello"))))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        3,
        "local generic closure の戻り値を使う user call は inner closure call の receiver/literal 保護に加えて outer call 用にも root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        3,
        "local generic closure の戻り値を使う user call は inner/outer 分の root_pop を行うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_lambda_argument() {
    let module = lower(
        r#"
        (defn accept-fn [f] 0)
        (defn main [] (accept-fn (fn [x] x)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "lambda literal 引数を使う user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "lambda literal 引数を使う user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_record_update_argument() {
    let module = lower(
        r#"
        (type Point (record (: x Int) (: y Int)))
        (defn consume-point [p] (Point.x p))
        (defn main []
          (let [p {Point x 1 y 2}]
            (consume-point {p | x 10})))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "record update 由来の record 引数を使う user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "record update 由来の record 引数を使う user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_auto_roots_vector_argument() {
    let module = lower(
        r#"
        (defn vector-len [v] (vector-length v))
        (defn main [] (vector-len (vector-new 2)))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert!(
        count_call_instr(&main_fn.body, 14) >= 1,
        "vector 引数の user call は root_push を使うべき: {:?}",
        main_fn.body
    );
    assert!(
        count_call_instr(&main_fn.body, 15) >= 1,
        "vector 引数の user call は root_pop を使うべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_does_not_root_int_argument() {
    let module = lower(
        r#"
        (defn double [n] (+ n n))
        (defn main [] (double 21))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();

    assert_eq!(
        count_call_instr(&main_fn.body, 14),
        0,
        "Int 引数だけの user call は root_push を使わないべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        0,
        "Int 引数だけの user call は root_pop を使わないべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_selectively_roots_heap_arguments() {
    let module = lower(
        r#"
        (defn consume-both [s n] (+ (string-length s) n))
        (defn main [] (consume-both "hello" 21))
        "#,
    );
    let main_fn = module.functions.iter().find(|f| f.name == "main").unwrap();
    let root_push_positions = call_positions(&main_fn.body, 14);

    assert_eq!(
        root_push_positions.len(),
        1,
        "heap 引数だけを 1 回 root_push するべき: {:?}",
        main_fn.body
    );
    assert_eq!(
        count_call_instr(&main_fn.body, 15),
        1,
        "heap 引数だけを 1 回 root_pop するべき: {:?}",
        main_fn.body
    );
}

#[test]
fn test_lower_user_call_conservatively_roots_opaque_call_result_argument() {
    let module = lower(
        r#"
        (defn consume-string [s] (string-length s))
        (defn forward [id x] (consume-string (id x)))
        "#,
    );
    let forward_fn = module
        .functions
        .iter()
        .find(|f| f.name == "forward")
        .unwrap();
    let root_push_positions = call_positions(&forward_fn.body, 14);
    let call_indirect_pos = forward_fn
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::CallIndirect(_)))
        .unwrap();
    let call_pos = forward_fn
        .body
        .iter()
        .enumerate()
        .find_map(|(idx, instr)| match instr {
            Instruction::Call(func_idx)
                if *func_idx != 14 && *func_idx != 15 && *func_idx != 16 =>
            {
                Some(idx)
            }
            _ => None,
        })
        .unwrap();

    assert_eq!(
        root_push_positions.len(),
        3,
        "opaque な inner call result を使う user call は receiver / arg / outer result を root_push するべき: {:?}",
        forward_fn.body
    );
    assert_eq!(
        count_call_instr(&forward_fn.body, 15),
        3,
        "opaque な inner call result を使う user call は 3 回 root_pop するべき: {:?}",
        forward_fn.body
    );
    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > call_indirect_pos && *pos < call_pos),
        "opaque な inner call result は outer direct call 前に root_push で保護されるべき: {:?}",
        forward_fn.body
    );
}
