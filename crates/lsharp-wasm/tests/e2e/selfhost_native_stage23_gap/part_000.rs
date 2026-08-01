use super::support::*;
use std::collections::{BTreeMap, BTreeSet};

fn selfhost_defn_max_nesting(source: &str, name: &str) -> usize {
    let marker = format!("(defn {name} ");
    let start = source
        .find(&marker)
        .unwrap_or_else(|| panic!("selfhost defn が見つからない: {name}"));
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut in_comment = false;

    for byte in source[start..].bytes() {
        if in_comment {
            if byte == b'\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b';' => in_comment = true,
            b'"' => in_string = true,
            b'(' => {
                depth += 1;
                max_depth = max_depth.max(depth);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return max_depth;
                }
            }
            _ => {}
        }
    }
    panic!("selfhost defn が閉じていない: {name}");
}

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
        86 => "CommandLineArgs",
        87 => "PrintString",
        88 => "ProcExit",
        89 => "WriteFile",
        90 => "WriteFileBytes",
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
        "CommandLineArgs",
        "Print",
        "PrintString",
        "ProcExit",
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
        "WriteFile",
        "WriteFileBytes",
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
fn test_selfhost_compiler_maps_native_cli_runtime_builtins_to_dedicated_opcodes() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "selfhost-native-cli-runtime-builtin-opcodes",
        &["CompilerBase.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Wasm.CompilerBase)

(defn main []
  (do
    (print (builtin-opcode 5217540237477903124))
    (print (builtin-opcode 2942060250258025265))
    (print (builtin-opcode 98761626082613))
    0))"#,
        &[],
    )
    .expect("native CLI runtime builtin opcode harness 実行に失敗");

    assert_eq!(
        parse_numeric_lines(&output),
        vec![86, 87, 88],
        "command-line-args/print-string/proc-exit は未使用の dedicated opcode に lower される必要がある"
    );
}

#[test]
fn test_selfhost_compiler_maps_not_equal_to_native_compare_opcode() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "selfhost-native-not-equal-builtin-opcode",
        &["CompilerBase.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Wasm.CompilerBase)

(defn main []
  (print (builtin-opcode 1084)))"#,
        &[],
    )
    .expect("native != builtin opcode harness 実行に失敗");

    assert_eq!(
        parse_numeric_lines(&output),
        vec![31],
        "!= は native compare opcode 31 に lower される必要がある"
    );
}

#[test]
fn test_native_codegen_x86_bundle_pads_declared_function_slot_before_next_function() {
    let source = std::fs::read_to_string(
        selfhost_package_root().join("src/Backend/Native/NativeCodegen.ls"),
    )
    .expect("NativeCodegen.ls 読み込みに失敗");
    let bundle_step = source
        .split("(defn generate-native-x86-64-bundle-loop-with-import-count-step")
        .nth(1)
        .and_then(|tail| tail.split("\n(defn ").next())
        .expect("x86 bundle step が存在すること");
    let padding_helper = source
        .split("(defn append-x86-function-padding-step-64")
        .nth(1)
        .and_then(|tail| tail.split("\n(defn ").next())
        .expect("x86 function slot padding step が存在すること");

    assert!(
        bundle_step.contains(
            "expected-end (+ (vector-get function-starts idx) (native-function-size-x86 func-meta functions))"
        ) && bundle_step.contains("actual-end (vector-length (ref-get result))")
            && bundle_step.contains("append-x86-function-padding result")
    );
    assert!(
        padding_helper.contains("append-native-bytes-rooted")
            && source.contains("(defn append-x86-function-padding [result padding]")
            && source.contains("continue-append-x86-function-padding")
    );
}

#[test]
fn test_selfhost_wasm_emit_print_string_appends_runtime_call() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "selfhost-wasm-emit-print-string-runtime-call",
        &["IR.ls", "WasiBackend.ls", "WasmEmit.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Wasm.WasmEmit)

(defn print-bytes-loop [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) len))))

(defn main []
  (let [bytes (emit-runtime-ir-instr (vector-new 4) 87 0)]
    (do
      (print (vector-length bytes))
      (print-bytes-loop bytes 0 (vector-length bytes))
      0)))"#,
        &[],
    )
    .expect("selfhost Wasm print-string emitter harness 実行に失敗");

    assert_eq!(
        parse_numeric_lines(&output),
        vec![4, 16, 10, 66, 0],
        "print-string opcode は 11 番目の runtime import への call と i64 のゼロ結果を出力する必要がある"
    );
}

#[test]
fn test_selfhost_wasm_function_body_keeps_print_string_result() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "selfhost-wasm-function-body-print-string-result",
        &["IR.ls", "WasiBackend.ls", "WasmEmit.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Wasm.WasmEmit)

(defn print-bytes-loop [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) len))))

(defn main []
  (let [instr0 (vector-push (vector-new 2) 87)
        instr (vector-push instr0 0)
        ir (vector-push (vector-new 4) instr)
        body (build-function-body ir)]
    (do
      (print (vector-length body))
      (print-bytes-loop body 0 (vector-length body))
      0)))"#,
        &[],
    )
    .expect("selfhost Wasm function body print-string harness 実行に失敗");

    assert_eq!(
        parse_numeric_lines(&output),
        vec![6, 0, 16, 10, 66, 0, 11],
        "function body emission は print-string の runtime call と i64 のゼロ結果を保持する必要がある"
    );
}

#[test]
fn test_selfhost_wasm_print_string_runtime_layout_uses_eleven_imports() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "selfhost-wasm-print-string-runtime-layout",
        &["IR.ls", "WasiBackend.ls", "WasmEmit.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Wasm.WasmEmit)

(defn main []
  (let [imports (emit-import-section-alloc-print-read-arg-concat-sub-print-string)
        code (emit-code-section-wasi-quad-functions-print-string (vector-new 0))
        import-count-idx (if (>= (vector-get imports 1) 128) 3 2)]
    (do
      (print (vector-get imports 0))
      (print (vector-get imports import-count-idx))
      (print (vector-length code))
      (print (vector-get code 6))
      0)))"#,
        &[],
    )
    .expect("selfhost Wasm print-string runtime layout harness 実行に失敗");

    assert_eq!(
        parse_numeric_lines(&output),
        vec![2, 11, 9, 10],
        "print-string runtime は 11 imports と _start -> import_count + function_count - 1 を維持する必要がある"
    );
}

#[test]
fn test_native_cli_runtime_extension_keeps_large_dispatch_nesting_bounded() {
    let source = std::fs::read_to_string(
        selfhost_package_root().join("src/Backend/Native/NativeCodegen.ls"),
    )
    .expect("NativeCodegen.ls 読み込みに失敗");
    let limits = [
        ("opcode-pushes-stack", 7),
        ("native-instr-size-x86", 35),
        ("x86-selfhost-helper-trailer-size", 24),
        ("is-selfhost-runtime-opcode-x86", 24),
        ("codegen-selfhost-runtime-bundle-x86", 25),
        (
            "generate-native-control-instr-bundle-loop-x86-with-context",
            35,
        ),
        ("native-selfhost-runtime-helper-tail-size-aarch64", 14),
        ("codegen-selfhost-runtime-bundle-aarch64-tail", 16),
        ("codegen-selfhost-runtime-bundle-aarch64", 16),
    ];

    for (name, limit) in limits {
        let actual = selfhost_defn_max_nesting(&source, name);
        assert!(
            actual <= limit,
            "{name} の式深度 {actual} が既存上限 {limit} を超えている。追加 opcode は小さい helper に分離する必要がある"
        );
    }

    let wasm_source =
        std::fs::read_to_string(selfhost_package_root().join("src/Backend/Wasm/WasmEmit.ls"))
            .expect("WasmEmit.ls 読み込みに失敗");
    assert!(
        selfhost_defn_max_nesting(&wasm_source, "emit-runtime-ir-instr-tail-high") <= 7,
        "native 専用 opcode の拒否は既存 Wasm dispatch の式深度を増やさないこと"
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
fn test_native_codegen_x86_cli_runtime_call_sites_resolve_helper_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-cli-runtime-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 1024
        argc-bytes (codegen-ir-instr-bundle-x86-with-import-count 86 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        print-string-bytes (codegen-ir-instr-bundle-x86-with-import-count 87 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)
        proc-exit-bytes (codegen-ir-instr-bundle-x86-with-import-count 88 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 1)]
    (do
      (print (x86-selfhost-command-line-args-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-print-string-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-proc-exit-helper-offset import-stub-offset import-count))
      (print (opcode-stack-delta 86 0 (vector-new 0)))
      (print (opcode-stack-delta 87 0 (vector-new 0)))
      (print (opcode-stack-delta 88 0 (vector-new 0)))
      (print (vector-length argc-bytes))
      (print-bytes-loop argc-bytes 0 (vector-length argc-bytes))
      (print (vector-length print-string-bytes))
      (print-bytes-loop print-string-bytes 0 (vector-length print-string-bytes))
      (print (vector-length proc-exit-bytes))
      (print-bytes-loop proc-exit-bytes 0 (vector-length proc-exit-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            5807, 5811, 5862, 1, 0, 0, 5, 232, 170, 18, 0, 0, 7, 81, 232, 173, 18, 0, 0, 89, 7, 81,
            232, 224, 18, 0, 0, 89,
        ],
        "x86_64 command-line-args/print-string/proc-exit call site は stack effect と trailer offset を一致させる必要がある"
    );
}

#[test]
fn test_native_codegen_x86_write_file_helpers_use_binary_file_abi() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-write-file-helper-abi",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 1024
        write-file-helper (emit-x86-selfhost-write-file-helper)
        write-file-bytes-helper (emit-x86-selfhost-write-file-bytes-helper)
        write-file-call (codegen-ir-instr-bundle-x86-with-import-count 89 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        write-file-bytes-call (codegen-ir-instr-bundle-x86-with-import-count 90 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 2)
        direct-write-file-call (ref-new (vector-new 0))
        _ (append-two-arg-helper-call-x86 direct-write-file-call 4660 30 2)]
    (do
      (print (is-selfhost-runtime-opcode-x86 89))
      (print (is-selfhost-runtime-opcode-x86 90))
      (print (opcode-stack-delta 89 0 (vector-new 0)))
      (print (opcode-stack-delta 90 0 (vector-new 0)))
      (print (x86-selfhost-proc-exit-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-write-file-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-write-file-bytes-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-helper-trailer-size import-count))
      (print (vector-length write-file-helper))
      (print-bytes-loop write-file-helper 0 16)
      (print-bytes-loop write-file-helper (- (vector-length write-file-helper) 11) (vector-length write-file-helper))
      (print (vector-length write-file-bytes-helper))
      (print-bytes-loop write-file-bytes-helper 0 16)
      (print-bytes-loop write-file-bytes-helper (- (vector-length write-file-bytes-helper) 13) (vector-length write-file-bytes-helper))
      (print (vector-length write-file-call))
      (print-bytes-loop write-file-call 0 (vector-length write-file-call))
      (print (vector-length write-file-bytes-call))
      (print-bytes-loop write-file-bytes-call 0 (vector-length write-file-bytes-call))
      (print (vector-length (ref-get direct-write-file-call)))
      (print-bytes-loop (ref-get direct-write-file-call) 0 (vector-length (ref-get direct-write-file-call)))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            1, 1, -1, -1, 5862, 5874, 6061, 2389, 187, 83, 65, 84, 65, 85, 65, 86, 73, 137, 230,
            73, 137, 244, 69, 49, 237, 76, 137, 244, 65, 94, 65, 93, 65, 92, 91, 195, 255, 83, 65,
            84, 65, 85, 65, 86, 65, 87, 73, 137, 230, 73, 137, 244, 69, 76, 137, 244, 65, 95, 65,
            94, 65, 93, 65, 92, 91, 195, 11, 72, 137, 198, 72, 137, 207, 232, 231, 18, 0, 0, 11,
            72, 137, 198, 72, 137, 207, 232, 162, 19, 0, 0, 11, 72, 137, 198, 72, 137, 207, 232, 52,
            18, 0, 0,
        ],
        "x86_64 write-file / write-file-bytes は binary ABI、trailer offset、raw byte write helper を一致させる必要がある",
    );
}
