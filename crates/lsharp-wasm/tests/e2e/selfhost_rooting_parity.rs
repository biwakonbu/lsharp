use super::support::*;

const OP_I64_CONST: i64 = 1;
const OP_SUBSTRING: i64 = 69;
const OP_STRING_CONCAT: i64 = 70;
const OP_ROOT_PUSH: i64 = 74;
const OP_ROOT_POP: i64 = 75;
const OP_REF_NEW: i64 = 56;
const OP_VECTOR_NEW: i64 = 54;
const OP_VECTOR_PUSH: i64 = 55;
const OP_MAP_NEW: i64 = 60;
const OP_MAP_INSERT: i64 = 62;
const OP_MAP_GET: i64 = 63;
const OP_CALL: i64 = 40;

fn escape_lsharp_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn compile_selfhost_ir_report(source: &str) -> (i64, Vec<(i64, i64)>) {
    let escaped = escape_lsharp_string(source);
    let harness = format!(
        r#"
(defn print-ir-loop [ir idx count]
  (if (>= idx count)
    0
    (let [instr (vector-get ir idx)]
      (do
        (print (vector-get instr 0))
        (print (vector-get instr 1))
        (print-ir-loop ir (+ idx 1) count)))))

(defn main []
  (let [source "{}"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        local-count (vector-get main-fn 1)
        main-ir (vector-get main-fn 2)
        instr-count (vector-length main-ir)]
    (do
      (print local-count)
      (print instr-count)
      (print-ir-loop main-ir 0 instr-count)
      0)))
"#,
        escaped
    );
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 2,
        "selfhost IR report は local-count / instr-count を含むべき: {:?}",
        lines
    );
    let local_count = lines[0]
        .parse::<i64>()
        .expect("selfhost IR report local-count は整数であること");
    let instr_count = lines[1]
        .parse::<usize>()
        .expect("selfhost IR report instr-count は整数であること");
    assert_eq!(
        lines.len(),
        2 + (instr_count * 2),
        "selfhost IR report 行数が命令数と一致しない: {:?}",
        lines
    );
    let instrs = lines[2..]
        .chunks_exact(2)
        .map(|chunk| {
            let opcode = chunk[0]
                .parse::<i64>()
                .expect("selfhost IR opcode は整数であること");
            let operand = chunk[1]
                .parse::<i64>()
                .expect("selfhost IR operand は整数であること");
            (opcode, operand)
        })
        .collect::<Vec<_>>();
    (local_count, instrs)
}

#[test]
fn test_e2e_selfhost_compiler_string_concat_auto_roots_arguments() {
    let (local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (string-concat "a" "b"))"#);
    let concat_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_STRING_CONCAT)
        .expect("selfhost string-concat lowering は concat opcode を含むべき");
    let pushes_before_concat = instrs[..concat_pos]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let pops_after_concat = instrs[concat_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        local_count >= 2,
        "selfhost string-concat auto-root は lhs/rhs spill 用 local を確保すべき: {:?}",
        instrs
    );
    assert!(
        pushes_before_concat >= 2,
        "selfhost string-concat は concat 前に 2 つの root_push を挿入すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_concat >= 2,
        "selfhost string-concat は concat 後に 2 つの root_pop を挿入すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_string_concat_roots_lhs_before_lowering_rhs() {
    let (_local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (string-concat (string-concat "a" "b") "c"))"#);
    let concat_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_STRING_CONCAT).then_some(idx))
        .collect::<Vec<_>>();
    let const_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_I64_CONST).then_some(idx))
        .collect::<Vec<_>>();
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        concat_positions.len() >= 2,
        "nested string-concat には inner/outer の 2 concat opcode が必要: {:?}",
        instrs
    );
    assert!(
        const_positions.len() >= 3,
        "nested string-concat には 3 つの string literal const が必要: {:?}",
        instrs
    );
    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > concat_positions[0] && *pos < const_positions[2]),
        "outer lhs の concat 結果は rhs literal を lowering する前に root_push されるべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_substring_auto_roots_source_string() {
    let (local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (substring "abcd" 1 3))"#);
    let substring_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_SUBSTRING)
        .expect("selfhost substring lowering は substring opcode を含むべき");
    let pushes_before_substring = instrs[..substring_pos]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let pops_after_substring = instrs[substring_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        local_count >= 1,
        "selfhost substring auto-root は source spill 用 local を確保すべき: {:?}",
        instrs
    );
    assert!(
        pushes_before_substring >= 1,
        "selfhost substring は call 前に source string を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_substring >= 1,
        "selfhost substring は call 後に source string を root_pop で解放すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_substring_roots_source_before_lowering_index_exprs() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [] (substring "abcd" (string-length (string-concat "x" "y")) 2))"#,
    );
    let const_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_I64_CONST).then_some(idx))
        .collect::<Vec<_>>();
    let root_push_pos = instrs
        .iter()
        .enumerate()
        .find_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .expect("substring rooting には root_push が必要");

    assert!(
        const_positions.len() >= 3,
        "substring source / index expr 用の const が不足している: {:?}",
        instrs
    );
    assert!(
        root_push_pos > const_positions[0] && root_push_pos < const_positions[1],
        "substring source は index expr の最初の const を lowering する前に root_push されるべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_ref_new_auto_roots_wrapped_value() {
    let (_local_count, instrs) = compile_selfhost_ir_report(r#"(defn main [] (ref-new "ab"))"#);
    let ref_new_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_REF_NEW)
        .expect("selfhost ref-new lowering は ref-new opcode を含むべき");
    let pushes_before_ref_new = instrs[..ref_new_pos]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let pops_after_ref_new = instrs[ref_new_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        pushes_before_ref_new >= 1,
        "selfhost ref-new は alloc 前に wrapped value を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_ref_new >= 1,
        "selfhost ref-new は alloc 後に wrapped value を root_pop で解放すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_ref_new_roots_inner_result_before_alloc() {
    let (_local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (ref-new (string-concat "a" "b")))"#);
    let concat_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_STRING_CONCAT)
        .expect("nested ref-new test には inner string-concat が必要");
    let ref_new_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_REF_NEW)
        .expect("nested ref-new test には ref-new opcode が必要");
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > concat_pos && *pos < ref_new_pos),
        "ref-new は inner allocating expr の結果を alloc 前に root_push で保護すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_vector_push_auto_roots_realloc_inputs() {
    let (_local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (vector-push (vector-new 0) "x"))"#);
    let vector_push_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_VECTOR_PUSH)
        .expect("selfhost vector-push lowering は vector-push opcode を含むべき");
    let pushes_before_vector_push = instrs[..vector_push_pos]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let pops_after_vector_push = instrs[vector_push_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        pushes_before_vector_push >= 2,
        "selfhost vector-push は realloc 前に receiver/value を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_vector_push >= 2,
        "selfhost vector-push は realloc 後に receiver/value を root_pop で解放すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_vector_push_roots_vector_before_lowering_value_expr() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [] (vector-push (vector-new 0) (string-concat "a" "b")))"#,
    );
    let vector_new_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_VECTOR_NEW)
        .expect("vector-push test には receiver vector-new が必要");
    let const_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_I64_CONST).then_some(idx))
        .collect::<Vec<_>>();
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        const_positions.len() >= 3,
        "vector-push receiver/value expr 用の const が不足している: {:?}",
        instrs
    );
    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > vector_new_pos && *pos < const_positions[1]),
        "vector-push は value expr の lowering 前に receiver を root_push で保護すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_vector_push_roots_value_result_before_alloc() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [] (vector-push (vector-new 0) (string-concat "a" "b")))"#,
    );
    let concat_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_STRING_CONCAT)
        .expect("vector-push test には inner string-concat が必要");
    let vector_push_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_VECTOR_PUSH)
        .expect("vector-push test には vector-push opcode が必要");
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > concat_pos && *pos < vector_push_pos),
        "vector-push は inner allocating value の結果を realloc 前に root_push で保護すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_map_insert_auto_roots_receiver_key_value() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [] (map-insert (map-new) (vector-new 0) (vector-new 0)))"#,
    );
    let map_insert_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_MAP_INSERT)
        .expect("selfhost map-insert lowering は map-insert opcode を含むべき");
    let pushes_before_map_insert = instrs[..map_insert_pos]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let pops_after_map_insert = instrs[map_insert_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        pushes_before_map_insert >= 3,
        "selfhost map-insert は receiver/key/value を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_map_insert >= 3,
        "selfhost map-insert は receiver/key/value を root_pop で解放すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_map_insert_roots_receiver_before_lowering_key_expr() {
    let (_local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (map-insert (map-new) (vector-new 0) 1))"#);
    let map_new_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_MAP_NEW)
        .expect("map-insert test には receiver map-new が必要");
    let vector_new_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_VECTOR_NEW)
        .expect("map-insert test には key vector-new が必要");
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > map_new_pos && *pos < vector_new_pos),
        "map-insert は key expr の lowering 前に receiver を root_push で保護すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_map_insert_roots_value_before_op() {
    let (_local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (map-insert (map-new) 1 (vector-new 0)))"#);
    let vector_new_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_VECTOR_NEW)
        .expect("map-insert test には value vector-new が必要");
    let map_insert_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_MAP_INSERT)
        .expect("map-insert test には map-insert opcode が必要");
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > vector_new_pos && *pos < map_insert_pos),
        "map-insert は value expr の結果を op 前に root_push で保護すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_map_get_auto_roots_receiver_and_key() {
    let (_local_count, instrs) =
        compile_selfhost_ir_report(r#"(defn main [] (map-get (map-new) (vector-new 0)))"#);
    let map_get_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_MAP_GET)
        .expect("selfhost map-get lowering は map-get opcode を含むべき");
    let pushes_before_map_get = instrs[..map_get_pos]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let pops_after_map_get = instrs[map_get_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        pushes_before_map_get >= 2,
        "selfhost map-get は receiver/key を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_map_get >= 2,
        "selfhost map-get は receiver/key を root_pop で解放すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_user_call_roots_first_arg_before_lowering_later_args() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"
(defn main []
  (keep-first (id "a") "b"))

(defn id [x] x)
(defn keep-first [x y] x)
"#,
    );
    let call_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_CALL).then_some(idx))
        .collect::<Vec<_>>();
    let const_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_I64_CONST).then_some(idx))
        .collect::<Vec<_>>();
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();

    assert!(
        call_positions.len() >= 2,
        "nested user-call test には inner/outer の 2 call opcode が必要: {:?}",
        instrs
    );
    assert!(
        const_positions.len() >= 2,
        "nested user-call test には 2 つの string literal const が必要: {:?}",
        instrs
    );
    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > call_positions[0] && *pos < const_positions[1]),
        "outer user call は最初の heap 引数を後続引数の lowering 前に root_push で保護すべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_user_call_auto_roots_arguments_until_call() {
    let (local_count, instrs) = compile_selfhost_ir_report(
        r#"
(defn main []
  (keep-first (id "a") "b"))

(defn id [x] x)
(defn keep-first [x y] x)
"#,
    );
    let call_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_CALL).then_some(idx))
        .collect::<Vec<_>>();
    let const_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_I64_CONST).then_some(idx))
        .collect::<Vec<_>>();
    let outer_call_pos = *call_positions
        .last()
        .expect("outer user-call test には call opcode が必要");
    let root_push_positions = instrs
        .iter()
        .enumerate()
        .filter_map(|(idx, (opcode, _))| (*opcode == OP_ROOT_PUSH).then_some(idx))
        .collect::<Vec<_>>();
    let pops_after_outer_call = instrs[outer_call_pos + 1..]
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        local_count >= 2,
        "user call auto-root は引数 spill 用 local を確保すべき: {:?}",
        instrs
    );
    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > const_positions[1] && *pos < outer_call_pos),
        "outer user call は call 前まで後続 heap 引数を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        pops_after_outer_call >= 2,
        "outer user call は call 後に引数 root を root_pop で解放すべき: {:?}",
        instrs
    );
}
