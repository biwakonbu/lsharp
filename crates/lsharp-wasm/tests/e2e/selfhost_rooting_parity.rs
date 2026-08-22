use super::support::*;
use std::collections::HashMap;

const OP_I64_CONST: i64 = 1;
const OP_SUBSTRING: i64 = 69;
const OP_STRING_CONCAT: i64 = 70;
const OP_ROOT_PUSH: i64 = 74;
const OP_ROOT_POP: i64 = 75;
const OP_ROOT_SET: i64 = 76;
const OP_REF_NEW: i64 = 56;
const OP_VECTOR_NEW: i64 = 54;
const OP_VECTOR_PUSH: i64 = 55;
const OP_MAP_NEW: i64 = 60;
const OP_MAP_INSERT: i64 = 62;
const OP_MAP_GET: i64 = 63;
const OP_FILE_EXISTS: i64 = 73;
const OP_CALL: i64 = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
enum SelfhostRootValue {
    Unknown,
    Slot(u64),
}

/// selfhost が出力した raw opcode 列について、root slot の lexical lifetime を検査する。
///
/// この helper は Rust lower の個数検査とは別に、selfhost compiler が生成する
/// `root_push → root_set → root_pop` の対象 slot を追跡する。slot を local に保存して
/// `root_pop` 後に再利用するケースは、個数だけでは検出できないため明示的に拒否する。
fn assert_selfhost_root_lifetime(instrs: &[(i64, i64)], context: &str) {
    let mut stack = Vec::new();
    let mut locals = HashMap::new();
    let mut active_slots = Vec::new();
    let mut next_slot = 0;

    let pop =
        |stack: &mut Vec<SelfhostRootValue>| stack.pop().unwrap_or(SelfhostRootValue::Unknown);

    for (index, (opcode, operand)) in instrs.iter().enumerate() {
        match *opcode {
            OP_I64_CONST => stack.push(SelfhostRootValue::Unknown),
            10 => stack.push(
                locals
                    .get(operand)
                    .cloned()
                    .unwrap_or(SelfhostRootValue::Unknown),
            ),
            11 => {
                let value = pop(&mut stack);
                locals.insert(*operand, value);
            }
            44 => {
                pop(&mut stack);
            }
            OP_ROOT_PUSH => {
                pop(&mut stack);
                let slot = next_slot;
                next_slot += 1;
                active_slots.push(slot);
                stack.push(SelfhostRootValue::Slot(slot));
            }
            OP_ROOT_POP => {
                assert!(
                    active_slots.pop().is_some(),
                    "{context}: instruction {index} が空の root stack を pop している: {instrs:?}"
                );
                stack.push(SelfhostRootValue::Unknown);
            }
            OP_ROOT_SET => {
                pop(&mut stack);
                let slot = pop(&mut stack);
                assert!(
                    !active_slots.is_empty(),
                    "{context}: instruction {index} が空の root stack に root_set している: {instrs:?}"
                );
                if let SelfhostRootValue::Slot(slot) = slot {
                    assert!(
                        active_slots.contains(&slot),
                        "{context}: instruction {index} が stale root slot {slot} を root_set に渡している: {instrs:?}"
                    );
                }
                stack.push(SelfhostRootValue::Unknown);
            }
            _ => {}
        }
    }

    assert!(
        active_slots.is_empty(),
        "{context}: function end に active root slot が残っている: {active_slots:?}, instrs={instrs:?}"
    );
}

fn escape_lsharp_string(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn parse_selfhost_ir_report_output(output: &str) -> (i64, Vec<(i64, i64)>) {
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

fn compile_selfhost_ir_report_for_function(
    source: &str,
    function_index: usize,
) -> (i64, Vec<(i64, i64)>) {
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
        main-fn (vector-get functions {})
        local-count (function-meta-local-count main-fn)
        main-ir (function-meta-ir main-fn)
        instr-count (vector-length main-ir)]
    (do
      (print local-count)
      (print instr-count)
      (print-ir-loop main-ir 0 instr-count)
      0)))
"#,
        escaped, function_index
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
    parse_selfhost_ir_report_output(&compile_and_run(&combined))
}

fn compile_selfhost_ir_report(source: &str) -> (i64, Vec<(i64, i64)>) {
    compile_selfhost_ir_report_for_function(source, 0)
}

fn compile_selfhost_cli_file_ir_report_for_function(
    path: &str,
    function_index: usize,
) -> (i64, Vec<(i64, i64)>) {
    let escaped_path = escape_lsharp_string(path);
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
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        data-ref (ref-new (vector-new 8))
        functions (compile-file-functions-with-cache "{}" 10 cache-ref parse-count-ref data-ref)
        main-fn (vector-get functions {})
        local-count (function-meta-local-count main-fn)
        main-ir (function-meta-ir main-fn)
        instr-count (vector-length main-ir)]
    (do
      (print local-count)
      (print instr-count)
      (print-ir-loop main-ir 0 instr-count)
      0)))
"#,
        escaped_path, function_index
    );
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    parse_selfhost_ir_report_output(&compile_and_run_with_dir(
        &combined,
        &selfhost_project_root(),
    ))
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
#[ignore]
fn test_debug_selfhost_compiler_root_set_map_insert_ir() {
    let (local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [] (let [slot0 (root_push 111) set-result (root_set slot0 (map-insert (map-new) 123 456)) rooted-map (root_pop)] 0))"#,
    );
    eprintln!(
        "root_set_map_insert locals={local_count} instrs={:?}",
        instrs
    );
    assert!(!instrs.is_empty());
}

#[test]
fn test_e2e_selfhost_compiler_root_set_consumes_allocating_map_insert_result() {
    let (local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [m k v] (let [slot (root_push m)] (do (root_set slot (map-insert m k v)) (root_pop))))"#,
    );
    let map_insert_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_MAP_INSERT)
        .expect("allocating root_set fixture は map-insert opcode を含むべき");
    let root_set_pos = instrs
        .iter()
        .position(|(opcode, _)| *opcode == OP_ROOT_SET)
        .expect("allocating root_set fixture は root_set opcode を含むべき");
    let root_pop_pos = instrs
        .iter()
        .rposition(|(opcode, _)| *opcode == OP_ROOT_POP)
        .expect("allocating root_set fixture は root_pop opcode を含むべき");

    assert!(
        local_count >= 4,
        "allocating root_set は source args と value spill 用 local を確保すべき: {:?}",
        instrs
    );
    assert!(
        map_insert_pos < root_set_pos,
        "root_set は allocating map-insert の結果を計算した後に実行すべき: {:?}",
        instrs
    );
    assert!(
        root_set_pos < root_pop_pos,
        "root_set は root slot を pop する前に実行すべき: {:?}",
        instrs
    );
    assert_selfhost_root_lifetime(&instrs, "allocating root_set");
}

#[test]
fn test_e2e_selfhost_compiler_root_lifetime_ledger_tracks_nested_map_safe_point() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"(defn main [] (let [slot (root_push (map-new))] (do (root_set slot (map-insert (map-new) 1 2)) (root_pop))))"#,
    );

    assert_selfhost_root_lifetime(&instrs, "nested map safe point");
}

#[test]
#[ignore]
fn test_debug_selfhost_compiler_zero_arg_tag_vector_builder_ir() {
    let (local_count, instrs) = compile_selfhost_ir_report_for_function(
        r#"(defn tag [] 24) (defn wrap [value] (vector-push (vector-push (vector-new 2) (tag)) value))"#,
        1,
    );
    eprintln!(
        "zero_arg_tag_vector_builder locals={local_count} instrs={:?}",
        instrs
    );
    assert!(!instrs.is_empty());
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

#[test]
fn test_e2e_selfhost_compiler_let_roots_heap_binding_before_lowering_later_let_init() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"
(defn main []
  (let [x (id "a")]
    (let [y (id "b")]
      x)))

(defn id [x] x)
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
        "nested let test には 2 つの id call opcode が必要: {:?}",
        instrs
    );
    assert!(
        const_positions.len() >= 2,
        "nested let test には 2 つの string literal const が必要: {:?}",
        instrs
    );
    assert!(
        root_push_positions
            .iter()
            .any(|pos| *pos > call_positions[0] && *pos < const_positions[1]),
        "outer let binding は inner let init の lowering 前に root_push で保護されるべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_nested_user_call_let_chain_keeps_file_exists_condition_lowerable() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"
(defn main [module-name source-root package-root]
  (let [nested-rel (id module-name)
        local-nested (keep-first source-root nested-rel)]
    (if (= (string-length package-root) 0)
      "D"
      (if (file-exists? local-nested)
        "L"
        (let [index-rel (id module-name)
              index-root (keep-first package-root ".lsharp/module-index")
              index-path (keep-first index-root index-rel)
              indexed-target (if (file-exists? index-path) (read-file index-path) "")
              stdlib-root (keep-first package-root "stdlib")
              stdlib-nested (keep-first stdlib-root nested-rel)]
          (if (> (string-length indexed-target) 0)
            (string-concat "I|" indexed-target)
            (if (file-exists? stdlib-nested) "S" "M")))))))

(defn id [x] x)
(defn keep-first [x y] x)
"#,
    );

    assert!(
        instrs.iter().any(|(opcode, _)| *opcode == OP_CALL),
        "nested user-call let chain は helper call を IR に残すべき: {:?}",
        instrs
    );
    assert!(
        instrs.iter().any(|(opcode, _)| *opcode == OP_FILE_EXISTS),
        "nested user-call let chain は file-exists? 条件まで lowering できるべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_actual_module_resolver_module_path_cache_state_keeps_calls_and_file_exists()
 {
    let (_local_count, instrs) =
        compile_selfhost_ir_report_for_function(selfhost_module("ModuleResolver.ls"), 30);

    assert!(
        instrs.iter().any(|(opcode, _)| *opcode == OP_CALL),
        "actual ModuleResolver.module-path-cache-state は helper call を IR に残すべき: {:?}",
        instrs
    );
    assert!(
        instrs.iter().any(|(opcode, _)| *opcode == OP_FILE_EXISTS),
        "actual ModuleResolver.module-path-cache-state は file-exists? 条件まで lowering できるべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_mode_main_multifile_module_resolver_function_keeps_calls_and_file_exists()
 {
    let (_local_count, instrs) =
        compile_selfhost_cli_file_ir_report_for_function("selfhost/src/App/Main.ls", 30);

    assert!(
        instrs.iter().any(|(opcode, _)| *opcode == OP_CALL),
        "multi-file CompilerMode compile の ModuleResolver.module-path-cache-state は helper call を IR に残すべき: {:?}",
        instrs
    );
    assert!(
        instrs.iter().any(|(opcode, _)| *opcode == OP_FILE_EXISTS),
        "multi-file CompilerMode compile の ModuleResolver.module-path-cache-state は file-exists? 条件まで lowering できるべき: {:?}",
        instrs
    );
}

#[test]
fn test_e2e_selfhost_compiler_let_chain_roots_final_body_before_root_pop_drops() {
    let (_local_count, instrs) = compile_selfhost_ir_report(
        r#"
(defn main [module-name source-root package-root]
  (let [nested-rel (string-concat module-name ".ls")
        local-nested (string-concat source-root nested-rel)]
    (if (= (string-length package-root) 0)
      "D"
      (if (file-exists? local-nested)
        "L"
        (let [index-rel (string-concat module-name ".path")
              index-root (string-concat package-root ".lsharp/module-index")
              index-path (string-concat index-root index-rel)
              indexed-target (if (file-exists? index-path) (read-file index-path) "")
              stdlib-root (string-concat package-root "stdlib")
              stdlib-nested (string-concat stdlib-root nested-rel)]
          (if (> (string-length indexed-target) 0)
            (string-concat "I|" indexed-target)
            (if (file-exists? stdlib-nested) "S" "M")))))))
"#,
    );
    let root_push_count = instrs
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_PUSH)
        .count();
    let root_pop_count = instrs
        .iter()
        .filter(|(opcode, _)| *opcode == OP_ROOT_POP)
        .count();

    assert!(
        root_push_count >= 4,
        "nested source-aware let chain は heap binding を root_push で保護すべき: {:?}",
        instrs
    );
    assert!(
        root_pop_count >= 4,
        "nested source-aware let chain は final body 後に root_pop を挿入すべき: {:?}",
        instrs
    );
}
