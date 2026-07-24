//! self-TCO と spill completeness の GC root 回帰

use super::*;

#[test]
fn test_lower_self_recursive_user_call_preserves_self_tco() {
    let module = lower(
        r#"
        (defn append-loop [dst idx count]
          (if (>= idx count)
            dst
            (append-loop (vector-push dst idx) (+ idx 1) count)))
        (defn main [] 0)
        "#,
    );
    let append_loop = module
        .functions
        .iter()
        .find(|func| func.name == "append-loop")
        .unwrap();

    assert!(
        append_loop
            .body
            .iter()
            .any(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty)),
        "自己末尾再帰は Loop に変換されるべき: {:?}",
        append_loop.body
    );
    assert_eq!(
        count_call_instr(&append_loop.body, 17),
        0,
        "自己末尾再帰は direct Call のまま残らないべき: {:?}",
        append_loop.body
    );
}

#[test]
fn test_lower_self_recursive_heap_param_roots_loop_entry_and_updates_slot() {
    let module = lower(
        r#"
        (defn append-loop [dst idx count]
          (if (>= idx count)
            dst
            (append-loop (vector-push dst idx) (+ idx 1) count)))
        (defn main [] 0)
        "#,
    );
    let append_loop = module
        .functions
        .iter()
        .find(|func| func.name == "append-loop")
        .unwrap();
    let loop_pos = append_loop
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();
    let root_push_positions = call_positions(&append_loop.body, 14);
    let root_set_positions = call_positions(&append_loop.body, 16);
    let backedge_pos = append_loop
        .body
        .iter()
        .rposition(|instr| matches!(instr, Instruction::Br(_)))
        .unwrap();
    assert!(
        !root_push_positions.is_empty(),
        "heap param を持つ self-TCO loop は root_push を使うべき: {:?}",
        append_loop.body
    );
    assert!(
        root_push_positions[0] < loop_pos,
        "heap param は loop entry 前に root_push で保護されるべき: {:?}",
        append_loop.body
    );
    assert_eq!(
        root_set_positions.len(),
        1,
        "heap param を更新する self-TCO loop は 1 回 root_set するべき: {:?}",
        append_loop.body
    );
    assert!(
        root_set_positions[0] < backedge_pos,
        "更新後の heap param は loop backedge 前に root_set で差し替えるべき: {:?}",
        append_loop.body
    );
}

#[test]
fn test_lower_bootstrap_append_bytes_preserves_self_tco_at_high_function_index() {
    let mut source = String::new();
    for i in 0..900 {
        source.push_str(&format!("(defn fn{i:04} [] {i})\n"));
    }
    source.push_str(
        r#"
        (defn bootstrap-append-bytes [dst src idx count]
          (if (>= idx count)
            dst
            (bootstrap-append-bytes
              (vector-push dst (vector-get src idx))
              src (+ idx 1) count)))
        (defn main [] 0)
        "#,
    );

    let module = lower(&source);
    let func_pos = module
        .functions
        .iter()
        .position(|func| func.name == "bootstrap-append-bytes")
        .unwrap();
    let append_loop = &module.functions[func_pos];
    let self_idx = module.imports.len() as u32 + func_pos as u32;

    assert!(
        append_loop
            .body
            .iter()
            .any(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty)),
        "高 function index でも bootstrap-append-bytes は Loop に変換されるべき: {:?}",
        append_loop.body
    );
    assert_eq!(
        count_call_instr(&append_loop.body, self_idx),
        0,
        "高 function index でも bootstrap-append-bytes の自己再帰 Call は残らないべき: {:?}",
        append_loop.body
    );
}

#[test]
fn test_lower_self_recursive_int_params_do_not_emit_root_updates() {
    let module = lower(
        r#"
        (defn countdown [n]
          (if (<= n 0)
            0
            (countdown (- n 1))))
        (defn main [] 0)
        "#,
    );
    let countdown = module
        .functions
        .iter()
        .find(|func| func.name == "countdown")
        .unwrap();

    assert!(
        countdown
            .body
            .iter()
            .any(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty)),
        "Int だけの自己末尾再帰も Loop に変換されるべき: {:?}",
        countdown.body
    );
    assert_eq!(
        count_call_instr(&countdown.body, 14),
        0,
        "Int param だけの self-TCO loop は root_push を使わないべき: {:?}",
        countdown.body
    );
    assert_eq!(
        count_call_instr(&countdown.body, 16),
        0,
        "Int param だけの self-TCO loop は root_set を使わないべき: {:?}",
        countdown.body
    );
}

#[test]
fn test_gc_spill_completeness_call_site_matrix_roots_heap_values() {
    let direct_module = lower(
        r#"
        (defn consume-string [s] (string-length s))
        (defn main []
          (let [s "hello"]
            (consume-string s)))
        "#,
    );
    let direct_main = direct_module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .unwrap();
    let direct_call = call_position(
        &direct_main.body,
        function_index(&direct_module, "consume-string"),
    );
    assert_rooted_safe_point(
        &direct_main.body,
        direct_call,
        "let heap local を渡す direct call",
    );
    assert_roots_balanced(&direct_main.body, "let heap local を渡す direct call");

    let trait_module = lower(
        r#"
        (trait (Measure a)
          (defn measure [x] 0))
        (impl (Measure String)
          (defn measure [x] (string-length x)))
        (defn main []
          (let [s "hello"]
            (measure s)))
        "#,
    );
    let trait_main = trait_module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .unwrap();
    let trait_call = call_position(
        &trait_main.body,
        function_index(&trait_module, "Measure_String_measure"),
    );
    assert_rooted_safe_point(
        &trait_main.body,
        trait_call,
        "let heap local を渡す trait dispatch",
    );
    assert_roots_balanced(&trait_main.body, "let heap local を渡す trait dispatch");

    let closure_module = lower(
        r#"
        (defn make-measure [] (fn [s] (string-length s)))
        (defn main []
          (let [f (make-measure)
                s "hello"]
            (f s)))
        "#,
    );
    let closure_main = closure_module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .unwrap();
    let closure_call = call_indirect_positions(&closure_main.body)[0];
    assert_rooted_safe_point(
        &closure_main.body,
        closure_call,
        "let heap local を渡す closure call",
    );
    assert_eq!(
        count_call_instr(&closure_main.body, ROOT_PUSH_IDX),
        2,
        "closure call は receiver と let heap local の両方を root_push するべき: {:?}",
        closure_main.body
    );
    assert_roots_balanced(&closure_main.body, "let heap local を渡す closure call");
}

#[test]
fn test_gc_spill_completeness_nested_call_results_are_rerooted() {
    let direct_module = lower(
        r#"
        (defn id [x] x)
        (defn consume-string [s] (string-length s))
        (defn main [] (consume-string (id "hello")))
        "#,
    );
    let direct_main = direct_module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .unwrap();
    let inner_direct = call_position(&direct_main.body, function_index(&direct_module, "id"));
    let outer_direct = call_position(
        &direct_main.body,
        function_index(&direct_module, "consume-string"),
    );
    assert_root_push_between(
        &direct_main.body,
        inner_direct,
        outer_direct,
        "opaque/generic direct call result",
    );
    assert_roots_balanced(&direct_main.body, "opaque/generic direct call result");

    let closure_module = lower(
        r#"
        (defn make-show [] (fn [s] (string-length s)))
        (defn main []
          (let [id (fn [x] x)
                f (make-show)]
            (f (id "hello"))))
        "#,
    );
    let closure_main = closure_module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .unwrap();
    let indirect_calls = call_indirect_positions(&closure_main.body);
    assert!(
        indirect_calls.len() >= 2,
        "inner/outer closure call の 2 safe point が必要: {:?}",
        closure_main.body
    );
    assert_root_push_between(
        &closure_main.body,
        indirect_calls[0],
        indirect_calls[1],
        "opaque/generic closure call result",
    );
    assert_roots_balanced(&closure_main.body, "opaque/generic closure call result");
}

#[test]
fn test_gc_spill_completeness_self_tco_updates_all_heap_roots_before_backedge() {
    let module = lower(
        r#"
        (defn swap-loop [(: a String) (: b String) n]
          (if (<= n 0)
            a
            (swap-loop b a (- n 1))))
        (defn main [] 0)
        "#,
    );
    let swap_loop = module
        .functions
        .iter()
        .find(|func| func.name == "swap-loop")
        .unwrap();
    let loop_pos = swap_loop
        .body
        .iter()
        .position(|instr| matches!(instr, Instruction::Loop(_) | Instruction::LoopEmpty))
        .unwrap();
    let backedge_pos = swap_loop
        .body
        .iter()
        .rposition(|instr| matches!(instr, Instruction::Br(_)))
        .unwrap();
    let root_push_positions = call_positions(&swap_loop.body, ROOT_PUSH_IDX);
    let root_set_positions = call_positions(&swap_loop.body, ROOT_SET_IDX);

    assert_eq!(
        root_push_positions.len(),
        2,
        "2 つの heap param は loop entry 前に root_push されるべき: {:?}",
        swap_loop.body
    );
    assert!(
        root_push_positions.iter().all(|pos| *pos < loop_pos),
        "heap param の root_push は loop entry 前に必要: {:?}",
        swap_loop.body
    );
    assert_eq!(
        root_set_positions.len(),
        2,
        "更新された 2 つの heap param は backedge 前に root_set されるべき: {:?}",
        swap_loop.body
    );
    assert!(
        root_set_positions.iter().all(|pos| *pos < backedge_pos),
        "heap param の root_set は loop backedge 前に必要: {:?}",
        swap_loop.body
    );
    assert_roots_balanced(&swap_loop.body, "self-TCO heap params");
}

#[test]
fn test_gc_spill_completeness_runtime_alloc_roots_let_and_pattern_values() {
    let let_module = lower(
        r#"
        (defn main []
          (let [s "hello"]
            (ref-new s)))
        "#,
    );
    let let_main = let_module
        .functions
        .iter()
        .find(|func| func.name == "main")
        .unwrap();
    let alloc_positions = call_positions(&let_main.body, ALLOC_IDX);
    assert!(
        alloc_positions.len() >= 2,
        "文字列 literal と ref-new の allocation が必要: {:?}",
        let_main.body
    );
    assert_rooted_safe_point(
        &let_main.body,
        alloc_positions[1],
        "let heap local を包む ref-new allocation",
    );
    assert_roots_balanced(&let_main.body, "let heap local を包む ref-new allocation");

    let pattern_module = lower(
        r#"
        (type (Box a) (Box a))
        (defn box-ref [b]
          (match b
            [(Box s) (ref-new s)]))
        (defn main [] 0)
        "#,
    );
    let box_ref = pattern_module
        .functions
        .iter()
        .find(|func| func.name == "box-ref")
        .unwrap();
    let ref_alloc = call_position(&box_ref.body, ALLOC_IDX);
    assert_rooted_safe_point(
        &box_ref.body,
        ref_alloc,
        "pattern-bound heap field を包む ref-new allocation",
    );
    assert_roots_balanced(
        &box_ref.body,
        "pattern-bound heap field を包む ref-new allocation",
    );
}
