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
        "CommandLineArg",
        "ReadFile",
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
fn test_native_codegen_x86_command_line_arg_and_read_file_call_sites_resolve_helper_offsets() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-runtime-helper-call-sites",
        r#"  (let [import-stub-offset 4096
        import-count 10
        current-offset 1024
        command-bytes (codegen-ir-instr-bundle-x86-with-import-count 67 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)
        read-bytes (codegen-ir-instr-bundle-x86-with-import-count 64 0 current-offset (vector-new 0) (vector-new 0) import-count import-stub-offset 0 0)]
    (do
      (print (x86-selfhost-command-line-arg-helper-offset import-stub-offset import-count))
      (print (x86-selfhost-read-file-helper-offset import-stub-offset import-count))
      (print (vector-length command-bytes))
      (print-bytes-loop command-bytes 0 (vector-length command-bytes))
      (print (vector-length read-bytes))
      (print-bytes-loop read-bytes 0 (vector-length read-bytes))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            4097, 4115, 7, 81, 232, 251, 11, 0, 0, 89, 7, 81, 232, 13, 12, 0, 0, 89,
        ],
        "x86_64 CommandLineArg/ReadFile call site は push rcx + call helper + pop rcx を実バイトで出す必要がある"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("CommandLineArg"),
        "selfhost x86_64 gap supported set から CommandLineArg を外したまま"
    );
}

#[test]
fn test_native_codegen_x86_runtime_helper_emitters_return_executable_byte_vectors() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-runtime-helper-bytes",
        r#"  (let [command-helper (emit-x86-selfhost-command-line-arg-helper)
        read-helper (emit-x86-selfhost-read-file-helper)]
    (do
      (print (vector-length command-helper))
      (print-bytes-loop command-helper 0 (vector-length command-helper))
      (print (vector-length read-helper))
      (print-bytes-loop read-helper 0 8)
      (print-bytes-loop read-helper (- (vector-length read-helper) 8) (vector-length read-helper))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![
            18, 72, 133, 192, 124, 10, 76, 57, 240, 125, 5, 73, 139, 4, 199, 195, 49, 192, 195,
            204, 83, 65, 84, 65, 85, 69, 49, 237, 49, 192, 65, 93, 65, 92, 91, 195,
        ],
        "x86_64 runtime helper emitters は実行可能な prologue/epilogue を持つ byte vector を返す必要がある"
    );
    assert!(
        supported_selfhost_native_opcodes_x86_64().contains("ReadFile"),
        "selfhost x86_64 gap supported set から ReadFile を外したまま"
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
            .any(|name| name == "CommandLineArg"),
        "selfhost x86_64 gap report から CommandLineArg は消えているべき: {:?}",
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
        !report
            .selfhost_unsupported_aarch64
            .iter()
            .any(|name| matches!(name.as_str(), "CommandLineArg" | "ReadFile")),
        "selfhost aarch64 gap report から CommandLineArg/ReadFile は消えているべき: {:?}",
        report.selfhost_unsupported_aarch64
    );
}
