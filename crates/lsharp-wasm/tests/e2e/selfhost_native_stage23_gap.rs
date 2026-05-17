use super::support::*;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
struct NativeStage23GapReport {
    entry_path: String,
    function_count: usize,
    instruction_count: usize,
    opcode_histogram: BTreeMap<String, usize>,
    unsupported_x86_64: Vec<String>,
    unsupported_aarch64: Vec<String>,
    selfhost_function_count: usize,
    selfhost_instruction_count: usize,
    selfhost_opcode_histogram: BTreeMap<String, usize>,
    selfhost_unsupported_x86_64: Vec<String>,
    selfhost_unsupported_aarch64: Vec<String>,
}

fn instruction_name(instr: &lsharp_ir::Instruction) -> String {
    format!("{instr:?}")
        .split(['(', ' ', '{'])
        .next()
        .unwrap_or("Unknown")
        .to_string()
}

fn supported_native_opcodes_x86_64() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "I64Const",
        "I64Add",
        "I64Sub",
        "I64Mul",
        "I64Div",
        "I64Rem",
        "I64Eq",
        "I64Ne",
        "I64LtS",
        "I64GtS",
        "I64LeS",
        "I64GeS",
        "I32Const",
        "I32Add",
        "I32Mul",
        "I32Load",
        "I32Store",
        "I32Load8U",
        "I32And",
        "I32Or",
        "I64Load",
        "I64Store",
        "MemoryCopy",
        "MemoryFill",
        "I32WrapI64",
        "I64ExtendI32S",
        "I64ExtendI32U",
        "LocalGet",
        "LocalSet",
        "Drop",
        // 制御フロー: emit-native (single-function) パスで対応済み
        "Call",
        "If",
        "IfEmpty",
        "Else",
        "End",
        "Block",
        "BlockEmpty",
        "Loop",
        "LoopEmpty",
        "Br",
        "BrIf",
    ])
}

fn supported_native_opcodes_aarch64() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "I64Const",
        "I64Add",
        "I64Sub",
        "I64Mul",
        "I64Div",
        "I64Rem",
        "I64Eq",
        "I64Ne",
        "I64LtS",
        "I64GtS",
        "I64LeS",
        "I64GeS",
        "I32Const",
        "I32Add",
        "I32Mul",
        "I32Load",
        "I32Store",
        "I32Load8U",
        "I32And",
        "I32Or",
        "I64Load",
        "I64Store",
        "MemoryCopy",
        "MemoryFill",
        "I32WrapI64",
        "I64ExtendI32S",
        "I64ExtendI32U",
        "LocalGet",
        "LocalSet",
        "Drop",
        // 制御フロー: emit-native (single-function) パスで対応済み
        "Call",
        "If",
        "IfEmpty",
        "Else",
        "End",
        "Block",
        "BlockEmpty",
        "Loop",
        "LoopEmpty",
        "Br",
        "BrIf",
    ])
}

fn selfhost_instruction_name(opcode: i64) -> String {
    match opcode {
        1 => "I64Const",
        2 => "F64Const",
        3 => "I32Const",
        10 => "LocalGet",
        11 => "LocalSet",
        20 => "I64Add",
        21 => "I64Sub",
        22 => "I64Mul",
        23 => "I64Div",
        24 => "I32Add",
        25 => "I32Mul",
        26 => "I32And",
        27 => "I32Or",
        28 => "I64Rem",
        30 => "I64Eq",
        31 => "I64Ne",
        32 => "I64LtS",
        33 => "I64GtS",
        34 => "I64LeS",
        35 => "I64GeS",
        36 => "I64ExtendI32S",
        37 => "I64ExtendI32U",
        38 => "I32WrapI64",
        40 => "Call",
        41 => "IfEmpty",
        42 => "BlockEmpty",
        43 => "End",
        44 => "Drop",
        45 => "I32Load",
        46 => "I32Store",
        47 => "I32Load8U",
        48 => "I64Load",
        49 => "I64Store",
        50 => "StringCharAt",
        51 => "StringLength",
        52 => "VectorLength",
        53 => "VectorGet",
        54 => "VectorNew",
        55 => "VectorPush",
        56 => "RefNew",
        57 => "RefGet",
        58 => "RefSet",
        59 => "Print",
        60 => "MapNew",
        61 => "MapSize",
        62 => "MapInsert",
        63 => "MapGet",
        64 => "ReadFile",
        65 => "MapContains",
        66 => "MapRemove",
        67 => "CommandLineArg",
        68 => "RuntimeHashString",
        69 => "Substring",
        70 => "StringConcat",
        71 => "And",
        72 => "Or",
        73 => "FileExists",
        74 => "RootPush",
        75 => "RootPop",
        76 => "RootSet",
        77 => "MemoryCopy",
        78 => "MemoryFill",
        79 => "Else",
        80 => "Br",
        81 => "BrIf",
        82 => "LoopEmpty",
        83 => "If",
        84 => "Block",
        85 => "Loop",
        _ => return format!("Opcode{opcode}"),
    }
    .to_string()
}

fn supported_selfhost_native_opcodes_x86_64() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "I64Const",
        "I32Const",
        "LocalGet",
        "LocalSet",
        "I64Add",
        "I64Sub",
        "I64Mul",
        "I64Div",
        "I32Add",
        "I32Mul",
        "I32And",
        "I32Or",
        "And",
        "Or",
        "I64Rem",
        "I64Eq",
        "I64Ne",
        "I64LtS",
        "I64GtS",
        "I64LeS",
        "I64GeS",
        "I64ExtendI32S",
        "I64ExtendI32U",
        "I32WrapI64",
        "Call",
        "IfEmpty",
        "BlockEmpty",
        "End",
        "Drop",
        "FileExists",
        "I32Load",
        "I32Store",
        "I32Load8U",
        "I64Load",
        "I64Store",
        "MemoryCopy",
        "MemoryFill",
        "Else",
        "Br",
        "BrIf",
        "LoopEmpty",
        "If",
        "Block",
        "Loop",
        "MapGet",
        "MapInsert",
        "MapNew",
        "MapSize",
        "CommandLineArg",
        "Print",
        "ReadFile",
        "RefGet",
        "RefNew",
        "RefSet",
        "StringCharAt",
        "StringConcat",
        "StringLength",
        "Substring",
        "VectorGet",
        "VectorLength",
        "VectorNew",
        "VectorPush",
        "RootPush",
        "RootPop",
        "RootSet",
    ])
}

fn run_x86_selfhost_runtime_helper_harness(fixture_name: &str, main_body: &str) -> Vec<i64> {
    let entry_source = format!(
        r#"(module Main)
(import Backend.Native.NativeCodegen)

(defn print-bytes-loop [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) len))))

(defn main []
{main_body})"#
    );
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        fixture_name,
        &["IR.ls", "NativeTarget.ls", "NativeCodegen.ls"],
        "Main.ls",
        &entry_source,
        &[],
    )
    .expect("x86 selfhost runtime helper harness 実行に失敗");
    parse_numeric_lines(&output)
}

#[test]
fn test_selfhost_ftable_uses_flat_vector_storage_for_large_tables() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "selfhost-ftable-flat-vector-large-table",
        &["CompilerBase.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Wasm.CompilerBase)

(defn register-loop [idx limit table]
  (if (>= idx limit)
    table
    (register-loop (+ idx 1) limit (ftable-register table (+ 1000 idx) (+ 2000 idx)))))

(defn main []
  (let [table (register-loop 0 300 (ftable-new))
        overridden (ftable-register table 1299 7777)]
    (do
      (print (vector-length table))
      (print (ftable-lookup table 1299))
      (print (vector-length overridden))
      (print (ftable-lookup overridden 1299))
      (print (ftable-lookup table 9999))
      0)))"#,
        &[],
    )
    .expect("flat vector ftable harness 実行に失敗");

    assert_eq!(
        parse_numeric_lines(&output),
        vec![602, 7777, 602, 7777, 0],
        "ftable は map helper 容量に依存せず、key/value の flat vector として大きい表と後勝ち更新を保持する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_command_line_arg_and_read_file_call_sites_resolve_helper_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-runtime-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 1024
        command-bytes (codegen-ir-instr-bundle-x86-with-import-count 67 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        read-bytes (codegen-ir-instr-bundle-x86-with-import-count 64 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        strlen-bytes (codegen-ir-instr-bundle-x86-with-import-count 51 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        char-at-bytes (codegen-ir-instr-bundle-x86-with-import-count 50 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        print-bytes (codegen-ir-instr-bundle-x86-with-import-count 59 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)]
    (do
      (print (x86-selfhost-command-line-arg-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-read-file-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-string-length-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-string-char-at-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-print-helper-offset import-stub-offset import-count))
      (print (vector-length command-bytes))
      (print-bytes-loop command-bytes 0 (vector-length command-bytes))
      (print (vector-length read-bytes))
      (print-bytes-loop read-bytes 0 (vector-length read-bytes))
      (print (vector-length strlen-bytes))
      (print-bytes-loop strlen-bytes 0 (vector-length strlen-bytes))
      (print (vector-length char-at-bytes))
      (print-bytes-loop char-at-bytes 0 (vector-length char-at-bytes))
      (print (vector-length print-bytes))
      (print-bytes-loop print-bytes 0 (vector-length print-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            4097, 4115, 4322, 4374, 4445, 7, 81, 232, 251, 11, 0, 0, 89, 7, 81, 232, 13, 12, 0, 0,
            89, 7, 81, 232, 220, 12, 0, 0, 89, 5, 232, 17, 13, 0, 0, 7, 81, 232, 87, 13, 0, 0, 89,
        ],
        "x86_64 CommandLineArg/ReadFile/StringLength/StringCharAt/Print call site は helper call を実バイトで出す必要がある"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("CommandLineArg"),
        "selfhost x86_64 gap supported set から CommandLineArg を外したまま"
    );
}

#[test]
fn test_native_codegen_x86_vector_and_ref_helper_call_sites_resolve_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-ref-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 2048
        vector-new-bytes (codegen-ir-instr-bundle-x86-with-import-count 54 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        vector-length-bytes (codegen-ir-instr-bundle-x86-with-import-count 52 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        vector-get-bytes (codegen-ir-instr-bundle-x86-with-import-count 53 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        vector-push-bytes (codegen-ir-instr-bundle-x86-with-import-count 55 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        ref-new-bytes (codegen-ir-instr-bundle-x86-with-import-count 56 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        ref-get-bytes (codegen-ir-instr-bundle-x86-with-import-count 57 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        ref-set-bytes (codegen-ir-instr-bundle-x86-with-import-count 58 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)]
    (do
      (print (x86-selfhost-vector-new-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-vector-length-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-vector-get-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-vector-push-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-ref-new-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-ref-get-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-ref-set-helper-offset import-stub-offset import-count))
      (print (vector-length vector-new-bytes))
      (print-bytes-loop vector-new-bytes 0 (vector-length vector-new-bytes))
      (print (vector-length vector-length-bytes))
      (print-bytes-loop vector-length-bytes 0 (vector-length vector-length-bytes))
      (print (vector-length vector-get-bytes))
      (print-bytes-loop vector-get-bytes 0 (vector-length vector-get-bytes))
      (print (vector-length vector-push-bytes))
      (print-bytes-loop vector-push-bytes 0 (vector-length vector-push-bytes))
      (print (vector-length ref-new-bytes))
      (print-bytes-loop ref-new-bytes 0 (vector-length ref-new-bytes))
      (print (vector-length ref-get-bytes))
      (print-bytes-loop ref-get-bytes 0 (vector-length ref-get-bytes))
      (print (vector-length ref-set-bytes))
      (print-bytes-loop ref-set-bytes 0 (vector-length ref-set-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            4547, 4666, 4683, 4716, 4921, 4994, 5012, 7, 81, 232, 189, 9, 0, 0, 89, 7, 81, 232, 52,
            10, 0, 0, 89, 5, 232, 70, 10, 0, 0, 5, 232, 103, 10, 0, 0, 7, 81, 232, 51, 11, 0, 0,
            89, 7, 81, 232, 124, 11, 0, 0, 89, 5, 232, 143, 11, 0, 0,
        ],
        "x86_64 vector/ref helper call sites は trailer offset を指す call を出す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_string_slice_concat_helper_call_sites_resolve_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-slice-concat-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 3072
        substring-bytes (codegen-ir-instr-bundle-x86-with-import-count 69 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 3)
        concat-bytes (codegen-ir-instr-bundle-x86-with-import-count 70 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)]
    (do
      (print (x86-selfhost-substring-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-string-concat-helper-offset import-stub-offset import-count))
      (print (vector-length substring-bytes))
      (print-bytes-loop substring-bytes 0 (vector-length substring-bytes))
      (print (vector-length concat-bytes))
      (print-bytes-loop concat-bytes 0 (vector-length concat-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            4995, 5140, 12, 72, 139, 149, 248, 255, 255, 255, 232, 119, 7, 0, 0, 5, 232, 15, 8, 0,
            0,
        ],
        "x86_64 substring/string-concat helper call sites は trailer offset を指す call を出す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_map_and_file_helper_call_sites_resolve_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-map-file-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 4096
        map-new-bytes (codegen-ir-instr-bundle-x86-with-import-count 60 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        map-size-bytes (codegen-ir-instr-bundle-x86-with-import-count 61 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        map-insert-bytes (codegen-ir-instr-bundle-x86-with-import-count 62 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 3)
        map-get-bytes (codegen-ir-instr-bundle-x86-with-import-count 63 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        file-exists-bytes (codegen-ir-instr-bundle-x86-with-import-count 73 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)]
    (do
      (print (x86-selfhost-map-new-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-map-size-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-map-insert-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-map-get-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-file-exists-helper-offset import-stub-offset import-count))
      (print (vector-length map-new-bytes))
      (print-bytes-loop map-new-bytes 0 (vector-length map-new-bytes))
      (print (vector-length map-size-bytes))
      (print-bytes-loop map-size-bytes 0 (vector-length map-size-bytes))
      (print (vector-length map-insert-bytes))
      (print-bytes-loop map-insert-bytes 0 (vector-length map-insert-bytes))
      (print (vector-length map-get-bytes))
      (print-bytes-loop map-get-bytes 0 (vector-length map-get-bytes))
      (print (vector-length file-exists-bytes))
      (print-bytes-loop file-exists-bytes 0 (vector-length file-exists-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            5335, 5407, 5424, 5485, 5547, 5, 232, 210, 4, 0, 0, 7, 81, 232, 25, 5, 0, 0, 89, 12,
            72, 139, 149, 248, 255, 255, 255, 232, 36, 5, 0, 0, 5, 232, 104, 5, 0, 0, 7, 81, 232,
            165, 5, 0, 0, 89,
        ],
        "x86_64 map/file helper call sites は trailer offset を指す call を出す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_runtime_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-runtime-helper-bytes",
        r#"  (let [command-helper (emit-x86-selfhost-command-line-arg-helper)
        read-helper (emit-x86-selfhost-read-file-helper)
        strlen-helper (emit-x86-selfhost-string-length-helper)
        char-at-helper (emit-x86-selfhost-string-char-at-helper)
        print-helper (emit-x86-selfhost-print-helper)]
    (do
      (print (vector-length command-helper))
      (print-bytes-loop command-helper 0 (vector-length command-helper))
      (print (vector-length read-helper))
      (print-bytes-loop read-helper 0 8)
      (print-bytes-loop read-helper (- (vector-length read-helper) 8) (vector-length read-helper))
      (print (vector-length strlen-helper))
      (print-bytes-loop strlen-helper 0 (vector-length strlen-helper))
      (print (vector-length char-at-helper))
      (print-bytes-loop char-at-helper 0 (vector-length char-at-helper))
      (print (vector-length print-helper))
      (print-bytes-loop print-helper 0 8)
      (print-bytes-loop print-helper (- (vector-length print-helper) 8) (vector-length print-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            18, 72, 133, 192, 124, 10, 76, 57, 224, 125, 5, 73, 139, 4, 199, 195, 49, 192, 195,
            207, 83, 65, 84, 65, 85, 69, 49, 237, 49, 192, 65, 93, 65, 92, 91, 195, 52, 72, 133,
            192, 116, 44, 121, 9, 72, 15, 186, 240, 63, 139, 64, 4, 195, 72, 61, 0, 0, 0, 64, 115,
            7, 76, 1, 240, 139, 64, 4, 195, 72, 49, 201, 128, 60, 8, 0, 116, 5, 72, 255, 193, 235,
            245, 72, 137, 200, 195, 49, 192, 195, 71, 72, 133, 192, 120, 63, 72, 133, 201, 116, 58,
            72, 133, 201, 121, 16, 72, 15, 186, 241, 63, 59, 65, 4, 115, 43, 15, 182, 68, 1, 8,
            195, 72, 129, 249, 0, 0, 0, 64, 115, 14, 76, 1, 241, 59, 65, 4, 115, 20, 15, 182, 68,
            1, 8, 195, 72, 129, 249, 0, 16, 0, 0, 114, 5, 15, 182, 4, 1, 195, 49, 192, 195, 102,
            83, 72, 131, 236, 32, 72, 137, 195, 49, 192, 72, 131, 196, 32, 91, 195,
        ],
        "x86_64 runtime helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("ReadFile"),
        "selfhost x86_64 gap supported set から ReadFile を外したまま"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("StringLength"),
        "selfhost x86_64 gap supported set から StringLength を外したまま"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("StringCharAt"),
        "selfhost x86_64 gap supported set から StringCharAt を外したまま"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("Print"),
        "selfhost x86_64 gap supported set から Print を外したまま"
    );
}

#[test]
fn test_native_codegen_x86_string_slice_concat_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-slice-concat-helper-bytes",
        r#"  (let [substring-helper (emit-x86-selfhost-substring-helper)
        concat-helper (emit-x86-selfhost-string-concat-helper)]
    (do
      (print (vector-length substring-helper))
      (print-bytes-loop substring-helper 0 8)
      (print-bytes-loop substring-helper (- (vector-length substring-helper) 8) (vector-length substring-helper))
      (print (vector-length concat-helper))
      (print-bytes-loop concat-helper 0 8)
      (print-bytes-loop concat-helper (- (vector-length concat-helper) 8) (vector-length concat-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            145, 83, 65, 84, 65, 85, 65, 86, 65, 65, 94, 65, 93, 65, 92, 91, 195, 195, 72, 133,
            201, 120, 18, 72, 129, 249, 93, 65, 92, 91, 195, 49, 192, 195,
        ],
        "x86_64 substring/string-concat helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_map_and_file_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-map-file-helper-bytes",
        r#"  (let [map-new-helper (emit-x86-selfhost-map-new-helper)
        map-size-helper (emit-x86-selfhost-map-size-helper)
        map-insert-helper (emit-x86-selfhost-map-insert-helper)
        map-get-helper (emit-x86-selfhost-map-get-helper)
        file-exists-helper (emit-x86-selfhost-file-exists-helper)]
    (do
      (print (vector-length map-new-helper))
      (print-bytes-loop map-new-helper 0 8)
      (print-bytes-loop map-new-helper (- (vector-length map-new-helper) 8) (vector-length map-new-helper))
      (print (vector-length map-size-helper))
      (print-bytes-loop map-size-helper 0 (vector-length map-size-helper))
      (print (vector-length map-insert-helper))
      (print-bytes-loop map-insert-helper 0 8)
      (print-bytes-loop map-insert-helper (- (vector-length map-insert-helper) 8) (vector-length map-insert-helper))
      (print (vector-length map-get-helper))
      (print-bytes-loop map-get-helper 0 8)
      (print-bytes-loop map-get-helper (- (vector-length map-get-helper) 8) (vector-length map-get-helper))
      (print (vector-length file-exists-helper))
      (print-bytes-loop file-exists-helper 0 8)
      (print-bytes-loop file-exists-helper (- (vector-length file-exists-helper) 8) (vector-length file-exists-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            72, 81, 49, 255, 190, 16, 255, 0, 0, 232, 63, 89, 195, 49, 192, 89, 195, 17, 72, 133,
            192, 121, 9, 72, 15, 186, 240, 63, 139, 64, 8, 195, 49, 192, 195, 61, 83, 72, 137, 211,
            72, 133, 219, 121, 232, 63, 91, 195, 49, 192, 91, 195, 62, 83, 65, 84, 72, 133, 201,
            121, 48, 91, 195, 49, 192, 65, 92, 91, 195, 84, 83, 65, 84, 72, 137, 227, 72, 133, 192,
            72, 137, 220, 65, 92, 91, 195,
        ],
        "x86_64 map/file helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
}

#[test]
fn test_native_codegen_x86_read_file_helper_uses_linux_syscalls() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-read-file-linux-syscalls",
        r#"  (let [read-helper (emit-x86-selfhost-read-file-helper)]
    (do
      (print (vector-get read-helper 56))
      (print (vector-get read-helper 57))
      (print (vector-get read-helper 58))
      (print (vector-get read-helper 59))
      (print (vector-get read-helper 60))
      (print (vector-get read-helper 98))
      (print (vector-get read-helper 99))
      (print (vector-get read-helper 100))
      (print (vector-get read-helper 101))
      (print (vector-get read-helper 111))
      (print (vector-get read-helper 112))
      (print (vector-get read-helper 113))
      (print (vector-get read-helper 114))
      (print (vector-get read-helper 115))
      (print (vector-get read-helper 140))
      (print (vector-get read-helper 141))
      (print (vector-get read-helper 142))
      (print (vector-get read-helper 143))
      (print (vector-get read-helper 144))
      (print (vector-get read-helper 169))
      (print (vector-get read-helper 170))
      (print (vector-get read-helper 171))
      (print (vector-get read-helper 172))
      (print (vector-get read-helper 173))
      (print (vector-get read-helper 192))
      (print (vector-get read-helper 193))
      (print (vector-get read-helper 194))
      (print (vector-get read-helper 195))
      (print (vector-get read-helper 196))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            184, 2, 0, 0, 0, // open
            34, 0, 0, 0, // MAP_PRIVATE | MAP_ANONYMOUS
            184, 9, 0, 0, 0, // mmap
            184, 0, 0, 0, 0, // read
            184, 3, 0, 0, 0, // close
            184, 3, 0, 0, 0, // close on failure
        ],
        "x86_64 Linux read-file helper は Linux syscall 番号と mmap flags を使う必要がある"
    );
}

#[test]
fn test_native_codegen_x86_string_char_at_oob_jumps_to_return_zero() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-char-at-oob-jump",
        r#"  (let [helper (emit-x86-selfhost-string-char-at-helper)]
    (do
      (print (vector-get helper 23))
      (print (vector-get helper 24))
      (let [target (+ 25 (vector-get helper 24))]
        (do
          (print (vector-get helper target))
          (print (vector-get helper (+ target 1)))
          (print (vector-get helper (+ target 2)))))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![115, 43, 49, 192, 195],
        "x86_64 string-char-at の tagged out-of-bounds 分岐は return-zero へ飛ぶ必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_push_helper_has_growth_path() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-push-helper-growth-path",
        r#"  (let [helper (emit-x86-selfhost-vector-push-helper)]
    (do
      (print (vector-length helper))
      (print (vector-get helper 0))
      (print (vector-get helper 1))
      (print (vector-get helper 2))
      (print (vector-get helper 3))
      (print (vector-get helper 89))
      (print (vector-get helper 90))
      (print (vector-get helper 91))
      (print (vector-get helper 92))
      (print (vector-get helper 93))
      (print (vector-get helper 94))
      (print (vector-get helper 95))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![205, 65, 84, 65, 85, 184, 9, 0, 0, 0, 15, 5],
        "x86_64 vector-push helper は capacity 超過時に Linux mmap で grow できる必要がある"
    );
}

#[test]
fn test_native_codegen_x86_i64_sub_depth_three_restores_previous_window() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-i64-sub-depth-three",
        r#"  (let [bytes (codegen-ir-instr-bundle-x86-with-import-count 21 0 1024 (vector-new 0) (vector-new 0) 0 0 0 3)]
    (do
      (print (vector-length bytes))
      (print (vector-get bytes 0))
      (print (vector-get bytes 1))
      (print (vector-get bytes 2))
      (print (vector-get bytes 3))
      (print (vector-get bytes 4))
      (print (vector-get bytes 5))
      (print (vector-get bytes 6))
      (print (vector-get bytes 7))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![13, 72, 41, 193, 72, 137, 200, 72, 139],
        "x86_64 i64.sub bundle は depth>=3 で演算後に下段 stack window を rcx へ復元する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_and_ref_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-ref-helper-bytes",
        r#"  (let [vector-new-helper (emit-x86-selfhost-vector-new-helper)
        vector-length-helper (emit-x86-selfhost-vector-length-helper)
        vector-get-helper (emit-x86-selfhost-vector-get-helper)
        vector-push-helper (emit-x86-selfhost-vector-push-helper)
        ref-new-helper (emit-x86-selfhost-ref-new-helper)
        ref-get-helper (emit-x86-selfhost-ref-get-helper)
        ref-set-helper (emit-x86-selfhost-ref-set-helper)]
    (do
      (print (vector-length vector-new-helper))
      (print-bytes-loop vector-new-helper 0 8)
      (print-bytes-loop vector-new-helper (- (vector-length vector-new-helper) 8) (vector-length vector-new-helper))
      (print (vector-length vector-length-helper))
      (print-bytes-loop vector-length-helper 0 (vector-length vector-length-helper))
      (print (vector-length vector-get-helper))
      (print-bytes-loop vector-get-helper 0 (vector-length vector-get-helper))
      (print (vector-length vector-push-helper))
      (print-bytes-loop vector-push-helper 0 (vector-length vector-push-helper))
      (print (vector-length ref-new-helper))
      (print-bytes-loop ref-new-helper 0 8)
      (print-bytes-loop ref-new-helper (- (vector-length ref-new-helper) 8) (vector-length ref-new-helper))
      (print (vector-length ref-get-helper))
      (print-bytes-loop ref-get-helper 0 (vector-length ref-get-helper))
      (print (vector-length ref-set-helper))
      (print-bytes-loop ref-set-helper 0 (vector-length ref-set-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            119, 81, 80, 72, 141, 52, 197, 16, 0, 63, 89, 195, 88, 49, 192, 89, 195, 17, 72, 133,
            192, 121, 9, 72, 15, 186, 240, 63, 139, 64, 8, 195, 49, 192, 195, 33, 72, 133, 201,
            121, 25, 72, 129, 249, 0, 240, 255, 255, 127, 16, 72, 15, 186, 241, 63, 59, 65, 8, 115,
            6, 72, 139, 68, 193, 16, 195, 49, 192, 195, 205, 65, 84, 65, 85, 72, 133, 201, 15, 137,
            185, 0, 0, 0, 72, 15, 186, 241, 63, 139, 81, 8, 68, 139, 65, 4, 68, 57, 194, 15, 130,
            141, 0, 0, 0, 80, 81, 65, 137, 212, 69, 137, 197, 69, 133, 237, 117, 8, 65, 189, 1, 0,
            0, 0, 235, 3, 69, 1, 237, 49, 255, 74, 141, 52, 237, 16, 0, 0, 0, 186, 3, 0, 0, 0, 65,
            186, 34, 0, 0, 0, 73, 199, 192, 255, 255, 255, 255, 69, 49, 201, 184, 9, 0, 0, 0, 15,
            5, 72, 133, 192, 120, 63, 199, 0, 2, 0, 0, 0, 68, 137, 104, 4, 68, 137, 96, 8, 72, 137,
            194, 72, 139, 52, 36, 72, 141, 118, 16, 72, 141, 120, 16, 68, 137, 225, 243, 72, 165,
            94, 88, 68, 137, 225, 72, 137, 68, 202, 16, 255, 193, 137, 74, 8, 72, 15, 186, 234, 63,
            72, 137, 208, 65, 93, 65, 92, 195, 72, 131, 196, 16, 65, 93, 65, 92, 49, 192, 195, 72,
            137, 68, 209, 16, 255, 194, 137, 81, 8, 72, 15, 186, 233, 63, 72, 137, 200, 65, 93, 65,
            92, 195, 65, 93, 65, 92, 49, 192, 195, 73, 81, 80, 72, 49, 255, 72, 199, 198, 63, 89,
            195, 88, 49, 192, 89, 195, 18, 72, 133, 192, 121, 10, 72, 15, 186, 240, 63, 72, 139,
            64, 8, 195, 49, 192, 195, 20, 72, 133, 201, 121, 12, 72, 15, 186, 241, 63, 72, 137, 65,
            8, 49, 192, 195, 49, 192, 195,
        ],
        "x86_64 vector/ref helper emitters は実行可能な byte vector を返す必要がある"
    );
    for opcode in [
        "VectorNew",
        "VectorLength",
        "VectorGet",
        "VectorPush",
        "RefNew",
        "RefGet",
        "RefSet",
    ] {
        assert!(
            supported_selfhost_native_opcodes_x86_64().contains(opcode),
            "selfhost x86_64 gap supported set から {opcode} を外したまま"
        );
    }
}

#[test]
fn test_native_codegen_x86_substring_uses_end_minus_start_length() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-substring-end-minus-start",
        r#"  (let [helper (emit-x86-selfhost-substring-helper)]
    (do
      (print (vector-length helper))
      (print (vector-get helper 36))
      (print (vector-get helper 37))
      (print (vector-get helper 38))
      (print (vector-get helper 39))
      (print (vector-get helper 40))
      (print (vector-get helper 41))
      (print (vector-get helper 42))
      (print (vector-get helper 43))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![145, 65, 137, 204, 41, 200, 65, 137, 197],
        "x86_64 substring helper は length=end-start を保存し、path/module 文字列を壊さないこと"
    );
}

#[test]
fn test_native_codegen_x86_ref_new_preserves_initial_value_across_syscall() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-ref-new-syscall-preserve",
        r#"  (let [helper (emit-x86-selfhost-ref-new-helper)]
    (do
      (print (vector-length helper))
      (print (vector-get helper 0))
      (print (vector-get helper 1))
      (print (vector-get helper 56))
      (print (vector-get helper 57))
      (print (vector-get helper 58))
      (print (vector-get helper 59))
      (print (vector-get helper 68))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![73, 81, 80, 90, 72, 137, 80, 88],
        "x86_64 ref-new helper は syscall が r11 を壊しても初期値を ref cell に保存する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_new_preserves_capacity_across_syscall() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-new-syscall-preserve",
        r#"  (let [helper (emit-x86-selfhost-vector-new-helper)]
    (do
      (print (vector-length helper))
      (print (vector-get helper 0))
      (print (vector-get helper 1))
      (print (vector-get helper 2))
      (print (vector-get helper 49))
      (print (vector-get helper 50))
      (print (vector-get helper 51))
      (print (vector-get helper 52))
      (print (vector-get helper 96))
      (print (vector-get helper 97))
      (print (vector-get helper 98))
      (print (vector-get helper 99))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![119, 81, 80, 72, 120, 63, 65, 91, 68, 137, 88, 4],
        "x86_64 vector-new helper は syscall が r11 を壊しても capacity を vector header に保存する必要がある"
    );
}

#[test]
fn test_native_stage23_gap_report_covers_targeted_call_drop_memory_control_corpus() {
    use lsharp_ir::{Function, Instruction, IrType, Module};

    let module = Module {
        functions: vec![Function {
            name: "targeted_stage23_corpus".to_string(),
            params: vec![],
            result: IrType::I64,
            locals: vec![IrType::I64, IrType::I32],
            body: vec![
                Instruction::I64Const(1),
                Instruction::Call(0),
                Instruction::Drop,
                Instruction::I32Const(0),
                Instruction::I32Load { offset: 0 },
                Instruction::Drop,
                Instruction::I32Const(0),
                Instruction::I32Const(1),
                Instruction::I32Store { offset: 4 },
                Instruction::I32Const(0),
                Instruction::I32Load8U { offset: 8 },
                Instruction::Drop,
                Instruction::I32Const(0),
                Instruction::I64Load { offset: 16 },
                Instruction::Drop,
                Instruction::I32Const(0),
                Instruction::I64Const(2),
                Instruction::I64Store { offset: 24 },
                Instruction::I32Const(0),
                Instruction::I32Const(8),
                Instruction::I32Const(4),
                Instruction::MemoryCopy,
                Instruction::I32Const(0),
                Instruction::I32Const(0),
                Instruction::I32Const(16),
                Instruction::MemoryFill,
                Instruction::BlockEmpty,
                Instruction::LoopEmpty,
                Instruction::I32Const(1),
                Instruction::BrIf(0),
                Instruction::Br(1),
                Instruction::End,
                Instruction::IfEmpty,
                Instruction::I64Const(3),
                Instruction::Drop,
                Instruction::Else,
                Instruction::I64Const(4),
                Instruction::Drop,
                Instruction::End,
                Instruction::I64Const(0),
            ],
            is_export: true,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let report = collect_native_stage23_gap_report(
        std::path::Path::new("stage23-targeted-call-drop-memory-control-corpus.ls"),
        &module,
    );
    let required = [
        "Call",
        "Drop",
        "I32Load",
        "I32Store",
        "I32Load8U",
        "I64Load",
        "I64Store",
        "MemoryCopy",
        "MemoryFill",
        "BlockEmpty",
        "LoopEmpty",
        "BrIf",
        "Br",
        "IfEmpty",
        "Else",
        "End",
    ];

    for opcode in required {
        assert!(
            report.opcode_histogram.contains_key(opcode),
            "targeted stage23 corpus に {opcode} が含まれていない: {:?}",
            report.opcode_histogram
        );
    }
    assert!(
        report.unsupported_x86_64.is_empty(),
        "targeted stage23 corpus の x86_64 gap は空であるべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        report.unsupported_aarch64.is_empty(),
        "targeted stage23 corpus の aarch64 gap は空であるべき: {:?}",
        report.unsupported_aarch64
    );
}

fn supported_selfhost_native_opcodes_aarch64() -> BTreeSet<&'static str> {
    let mut supported = supported_selfhost_native_opcodes_x86_64();
    supported.insert("CommandLineArg");
    supported.insert("FileExists");
    supported.insert("MapGet");
    supported.insert("MapInsert");
    supported.insert("MapNew");
    supported.insert("MapSize");
    supported.insert("Print");
    supported.insert("ReadFile");
    supported.insert("RefGet");
    supported.insert("RefNew");
    supported.insert("RefSet");
    supported.insert("StringCharAt");
    supported.insert("StringConcat");
    supported.insert("StringLength");
    supported.insert("Substring");
    supported.insert("VectorGet");
    supported.insert("VectorLength");
    supported.insert("VectorNew");
    supported.insert("VectorPush");
    supported
}

fn collect_native_stage23_gap_report(
    entry_path: &std::path::Path,
    module: &lsharp_ir::Module,
) -> NativeStage23GapReport {
    let mut opcode_histogram = BTreeMap::new();
    let mut instruction_count = 0usize;
    for function in &module.functions {
        for instr in &function.body {
            instruction_count += 1;
            let name = instruction_name(instr);
            *opcode_histogram.entry(name).or_insert(0) += 1;
        }
    }

    let supported_x86_64 = supported_native_opcodes_x86_64();
    let supported_aarch64 = supported_native_opcodes_aarch64();
    let opcode_names: Vec<String> = opcode_histogram.keys().cloned().collect();
    let unsupported_x86_64 = opcode_names
        .iter()
        .filter(|name| !supported_x86_64.contains(name.as_str()))
        .cloned()
        .collect();
    let unsupported_aarch64 = opcode_names
        .iter()
        .filter(|name| !supported_aarch64.contains(name.as_str()))
        .cloned()
        .collect();

    NativeStage23GapReport {
        entry_path: entry_path.display().to_string(),
        function_count: module.functions.len(),
        instruction_count,
        opcode_histogram,
        unsupported_x86_64,
        unsupported_aarch64,
        selfhost_function_count: 0,
        selfhost_instruction_count: 0,
        selfhost_opcode_histogram: BTreeMap::new(),
        selfhost_unsupported_x86_64: Vec::new(),
        selfhost_unsupported_aarch64: Vec::new(),
    }
}

fn parse_numeric_lines(output: &str) -> Vec<i64> {
    output
        .trim()
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(
                    trimmed
                        .parse::<i64>()
                        .unwrap_or_else(|_| panic!("数値行の parse に失敗: {trimmed}")),
                )
            }
        })
        .collect()
}

fn run_selfhost_main_function_payload_opcode_harness() -> (usize, BTreeMap<String, usize>) {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "native-stage23-selfhost-opcodes",
        SELFHOST_APP_MAIN_REPRESENTATIVE_MODULES,
        "src/App/HarnessMain.ls",
        r#"(module App.HarnessMain)
(import App.CompilerMode)

(defn print-function-opcodes [functions idx len]
  (if (>= idx len)
    0
    (let [func (vector-get functions idx)
          ir (vector-get func 2)]
      (do
        (print-ir-opcodes ir 0 (vector-length ir))
        (print-function-opcodes functions (+ idx 1) len)))))

(defn print-ir-opcodes [ir idx len]
  (if (>= idx len)
    0
    (let [instr (vector-get ir idx)]
      (do
        (print (vector-get instr 0))
        (print-ir-opcodes ir (+ idx 1) len)))))

(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        payload (compile-file-functions-payload-with-cache "src/App/Main.ls" 10 cache-ref parse-count-ref)
        functions (vector-get payload 0)]
    (do
      (print (vector-length functions))
      (print-function-opcodes functions 0 (vector-length functions))
      0)))"#,
        &[],
    )
    .expect("selfhost opcode harness 実行に失敗");

    let lines = parse_numeric_lines(&output);
    assert!(!lines.is_empty(), "selfhost opcode harness の出力が空");
    let function_count = usize::try_from(lines[0]).expect("function count が負値");
    let mut histogram = BTreeMap::new();
    for opcode in &lines[1..] {
        *histogram
            .entry(selfhost_instruction_name(*opcode))
            .or_insert(0) += 1;
    }
    (function_count, histogram)
}

fn collect_selfhost_native_stage23_gap_report(
    entry_path: &std::path::Path,
) -> NativeStage23GapReport {
    let compile_entry_path = entry_path.to_path_buf();
    let mut report = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        let module = lsharp_ir::compile_multi_file(&compile_entry_path)
            .expect("selfhost App/Main.ls の multi-file compile に失敗");
        collect_native_stage23_gap_report(&compile_entry_path, &module)
    });

    let (selfhost_function_count, selfhost_opcode_histogram) =
        run_selfhost_main_function_payload_opcode_harness();
    let selfhost_instruction_count = selfhost_opcode_histogram.values().sum();
    let opcode_names: Vec<String> = selfhost_opcode_histogram.keys().cloned().collect();
    let supported_x86_64 = supported_selfhost_native_opcodes_x86_64();
    let supported_aarch64 = supported_selfhost_native_opcodes_aarch64();
    let selfhost_unsupported_x86_64 = opcode_names
        .iter()
        .filter(|name| !supported_x86_64.contains(name.as_str()))
        .cloned()
        .collect();
    let selfhost_unsupported_aarch64 = opcode_names
        .iter()
        .filter(|name| !supported_aarch64.contains(name.as_str()))
        .cloned()
        .collect();

    report.selfhost_function_count = selfhost_function_count;
    report.selfhost_instruction_count = selfhost_instruction_count;
    report.selfhost_opcode_histogram = selfhost_opcode_histogram;
    report.selfhost_unsupported_x86_64 = selfhost_unsupported_x86_64;
    report.selfhost_unsupported_aarch64 = selfhost_unsupported_aarch64;
    report
}

fn json_escape(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

fn render_json_string_list(values: &[String]) -> String {
    let quoted: Vec<String> = values
        .iter()
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect();
    format!("[{}]", quoted.join(", "))
}

fn render_json_histogram(histogram: &BTreeMap<String, usize>) -> String {
    let entries: Vec<String> = histogram
        .iter()
        .map(|(name, count)| format!("    \"{}\": {}", json_escape(name), count))
        .collect();
    format!("{{\n{}\n  }}", entries.join(",\n"))
}

fn write_native_stage23_gap_report(
    path: &std::path::Path,
    report: &NativeStage23GapReport,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("gap report dir 作成失敗: {e}"))?;
    }
    let supported_x86_64 = supported_native_opcodes_x86_64()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let supported_aarch64 = supported_native_opcodes_aarch64()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let supported_selfhost_x86_64 = supported_selfhost_native_opcodes_x86_64()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let supported_selfhost_aarch64 = supported_selfhost_native_opcodes_aarch64()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let json = format!(
        "{{\n  \"entry_path\": \"{}\",\n  \"function_count\": {},\n  \"instruction_count\": {},\n  \"selfhost_function_count\": {},\n  \"selfhost_instruction_count\": {},\n  \"supported_x86_64\": {},\n  \"supported_aarch64\": {},\n  \"supported_selfhost_x86_64\": {},\n  \"supported_selfhost_aarch64\": {},\n  \"unsupported_x86_64\": {},\n  \"unsupported_aarch64\": {},\n  \"selfhost_unsupported_x86_64\": {},\n  \"selfhost_unsupported_aarch64\": {},\n  \"opcode_histogram\": {},\n  \"selfhost_opcode_histogram\": {}\n}}\n",
        json_escape(&report.entry_path),
        report.function_count,
        report.instruction_count,
        report.selfhost_function_count,
        report.selfhost_instruction_count,
        render_json_string_list(&supported_x86_64),
        render_json_string_list(&supported_aarch64),
        render_json_string_list(&supported_selfhost_x86_64),
        render_json_string_list(&supported_selfhost_aarch64),
        render_json_string_list(&report.unsupported_x86_64),
        render_json_string_list(&report.unsupported_aarch64),
        render_json_string_list(&report.selfhost_unsupported_x86_64),
        render_json_string_list(&report.selfhost_unsupported_aarch64),
        render_json_histogram(&report.opcode_histogram),
        render_json_histogram(&report.selfhost_opcode_histogram),
    );
    std::fs::write(path, json).map_err(|e| format!("gap report 書き込み失敗: {e}"))
}

fn maybe_write_native_stage23_gap_report(report: &NativeStage23GapReport) -> Result<(), String> {
    let Some(path) = std::env::var_os("LSHARP_NATIVE_STAGE23_GAP_REPORT") else {
        return Ok(());
    };
    write_native_stage23_gap_report(&std::path::PathBuf::from(path), report)
}

/// V2-08: representative build entry の IR opcode gap を actual stage23 blocker report として固定する。
#[test]
#[ignore]
fn test_e2e_native_actual_stage23_gap_report_for_representative_entry() {
    let entry_path = selfhost_main_path();
    let report = collect_selfhost_native_stage23_gap_report(&entry_path);

    maybe_write_native_stage23_gap_report(&report)
        .expect("actual-stage23 gap report の書き出しに失敗");

    assert!(
        report.function_count > 0,
        "representative build entry の lowered function が 0"
    );
    assert!(
        report.instruction_count > 0,
        "representative build entry の lowered instruction が 0"
    );
    assert!(
        report.selfhost_function_count > 0,
        "representative selfhost function-meta payload の関数数が 0"
    );
    assert!(
        report.selfhost_instruction_count > 0,
        "representative selfhost function-meta payload の命令数が 0"
    );
    // 制御フロー対応後: gap は空になるため "not empty" アサーションは削除済み
    // (supported set に Call/If/Else/End/Block/Loop/Br/BrIf を追加)

    // 制御フロー opcodes が gap から消えていることを確認
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "Call"
                    | "If"
                    | "IfEmpty"
                    | "Else"
                    | "End"
                    | "Block"
                    | "BlockEmpty"
                    | "Loop"
                    | "LoopEmpty"
                    | "Br"
                    | "BrIf"
            )
        }),
        "x86_64 gap report から制御フロー opcodes は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "Call"
                    | "If"
                    | "IfEmpty"
                    | "Else"
                    | "End"
                    | "Block"
                    | "BlockEmpty"
                    | "Loop"
                    | "LoopEmpty"
                    | "Br"
                    | "BrIf"
            )
        }),
        "aarch64 gap report から制御フロー opcodes は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report
            .unsupported_x86_64
            .iter()
            .any(|name| name == "LocalGet" || name == "LocalSet" || name == "Drop"),
        "x86_64 gap report から LocalGet/LocalSet/Drop は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report
            .unsupported_aarch64
            .iter()
            .any(|name| name == "LocalGet" || name == "LocalSet" || name == "Drop"),
        "aarch64 gap report から LocalGet/LocalSet/Drop は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Const"
                    | "I32Add"
                    | "I32Mul"
                    | "I32And"
                    | "I32Or"
                    | "I32WrapI64"
                    | "I64ExtendI32S"
                    | "I64ExtendI32U"
            )
        }),
        "x86_64 gap report から i32 core opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Const"
                    | "I32Add"
                    | "I32Mul"
                    | "I32And"
                    | "I32Or"
                    | "I32WrapI64"
                    | "I64ExtendI32S"
                    | "I64ExtendI32U"
            )
        }),
        "aarch64 gap report から i32 core opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Load"
                    | "I32Store"
                    | "I32Load8U"
                    | "I64Load"
                    | "I64Store"
                    | "MemoryCopy"
                    | "MemoryFill"
            )
        }),
        "x86_64 gap report から memory opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Load"
                    | "I32Store"
                    | "I32Load8U"
                    | "I64Load"
                    | "I64Store"
                    | "MemoryCopy"
                    | "MemoryFill"
            )
        }),
        "aarch64 gap report から memory opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| matches!(
            name.as_str(),
            "I64Add" | "I64Sub" | "I64Mul" | "I64Div" | "I64Rem"
        )),
        "x86_64 gap report から I64Add/I64Sub/I64Mul/I64Div/I64Rem は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| matches!(
            name.as_str(),
            "I64Add" | "I64Sub" | "I64Mul" | "I64Div" | "I64Rem"
        )),
        "aarch64 gap report から I64Add/I64Sub/I64Mul/I64Div/I64Rem は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I64Eq" | "I64Ne" | "I64LtS" | "I64GtS" | "I64LeS" | "I64GeS"
            )
        }),
        "x86_64 gap report から主要 i64 compare opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I64Eq" | "I64Ne" | "I64LtS" | "I64GtS" | "I64LeS" | "I64GeS"
            )
        }),
        "aarch64 gap report から主要 i64 compare opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_x86_64
            .iter()
            .any(|name| matches!(name.as_str(), "And" | "Or")),
        "selfhost x86_64 gap report から logical and/or は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(name.as_str(), "And" | "Or")),
        "selfhost aarch64 gap report から logical and/or は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(name.as_str(), "RootPush" | "RootPop" | "RootSet")),
        "selfhost aarch64 gap report から root ops は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(
                name.as_str(),
                "CommandLineArg" | "ReadFile" | "StringCharAt" | "StringLength"
            )),
        "selfhost aarch64 gap report から command-line-arg/read-file/string-char-at/string-length は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| name == "Print"),
        "selfhost aarch64 gap report から Print は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(
                name.as_str(),
                "VectorGet" | "VectorLength" | "VectorNew" | "VectorPush"
            )),
        "selfhost aarch64 gap report から vector-get/vector-length/vector-new/vector-push は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        report.unsupported_aarch64.is_empty(),
        "aarch64 lowered IR の native unsupported blocker は 0 であるべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        report.selfhost_unsupported_aarch64.is_empty(),
        "aarch64 selfhost function-meta の native unsupported blocker は 0 であるべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
    assert!(
        !report
            .selfhost_unsupported_x86_64
            .iter()
            .any(|name| matches!(name.as_str(), "RootPush" | "RootPop" | "RootSet")),
        "selfhost x86_64 gap report から root ops は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
}

/// V2-09: representative actual stage23 gap report は aarch64 selfhost parity の残 blocker を 0 にする。
#[test]
#[ignore]
fn test_e2e_native_actual_stage23_gap_report_has_zero_aarch64_selfhost_blockers() {
    let entry_path = selfhost_main_path();
    let report = collect_selfhost_native_stage23_gap_report(&entry_path);

    assert!(
        report.unsupported_aarch64.is_empty(),
        "aarch64 lowered IR の native unsupported blocker が残っている: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        report.selfhost_unsupported_aarch64.is_empty(),
        "aarch64 selfhost function-meta の native unsupported blocker が残っている: {:?}",
        report.selfhost_unsupported_aarch64
    );
}

#[test]
#[ignore]
fn test_e2e_native_actual_stage23_gap_report_includes_selfhost_runtime_blockers() {
    let entry_path = selfhost_main_path();
    let report = collect_selfhost_native_stage23_gap_report(&entry_path);

    assert!(
        !report
            .selfhost_unsupported_x86_64
            .iter()
            .any(|name| matches!(
                name.as_str(),
                "CommandLineArg" | "StringCharAt" | "StringLength"
            )),
        "selfhost x86_64 gap report から CommandLineArg/StringCharAt/StringLength は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
    assert!(
        report
            .selfhost_unsupported_x86_64
            .iter()
            .all(|name| name != "ReadFile"),
        "selfhost x86_64 gap report から ReadFile は消えているべき: {:?}",
        report.selfhost_unsupported_x86_64
    );
    assert!(
        !report.selfhost_unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "CommandLineArg" | "ReadFile" | "StringCharAt" | "StringLength"
            )
        }),
        "selfhost aarch64 gap report から CommandLineArg/ReadFile/StringCharAt/StringLength は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
}
