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
            119, 81, 72, 137, 198, 72, 193, 230, 3, 144, 144, 144, 144, 144, 144, 144, 144, 17, 72, 133,
            192, 121, 9, 72, 15, 186, 240, 63, 139, 64, 8, 195, 49, 192, 195, 47, 72, 133, 201,
            121, 39, 72, 129, 249, 0, 240, 255, 255, 127, 30, 72, 15, 186, 241, 63, 72, 137, 202,
            72, 193, 234, 47, 117, 16, 131, 57, 2, 117, 11, 59, 65, 8, 115, 6, 72, 139, 68, 193,
            16, 195, 49, 192, 195,
            205, 65, 84, 65, 85, 72, 133, 201, 15, 137, 183, 0, 0, 0, 72, 15, 186, 241, 63, 139, 81,
            8, 68, 139, 97, 4, 68, 57, 226, 15, 130, 139, 0, 0, 0, 80, 81, 69, 137, 229, 65,
            137, 212, 69, 133, 237, 117, 8, 65, 189, 1, 0, 0, 0, 235, 3, 69, 1, 237, 73, 139,
            6, 74, 141, 60, 237, 16, 0, 0, 0, 72, 1, 199, 73, 59, 126, 8, 73, 137, 62, 72,
            186, 0, 0, 0, 0, 0, 0, 0, 128, 72, 15, 71, 194, 72, 133, 192, 120, 64, 76, 1,
            240, 199, 0, 2, 0, 0, 0, 68, 137, 104, 4, 68, 137, 96, 8, 72, 146, 72, 139, 52,
            36, 72, 141, 118, 16, 72, 141, 122, 16, 68, 137, 225, 243, 72, 165, 94, 88, 68, 137, 225,
            72, 137, 68, 202, 16, 255, 193, 137, 74, 8, 72, 15, 186, 234, 63, 72, 146, 65, 93, 65,
            92, 195, 72, 131, 196, 16, 65, 93, 65, 92, 49, 192, 195, 72, 137, 68, 209, 16, 255, 194,
            137, 81, 8, 72, 15, 186, 233, 63, 72, 137, 200, 65, 93, 65, 92, 195, 65, 93, 65, 92,
            49, 192, 195, 144, 144, 73, 81, 73, 137, 195, 73, 139, 14, 72, 89, 195, 49, 192, 89, 195,
            144, 144, 18, 72, 133, 192, 121, 10, 72, 15, 186, 240, 63, 72, 139, 64, 8, 195, 49, 192,
            195, 20, 72, 133, 201, 121, 12, 72, 15, 186, 241, 63, 72, 137, 65, 8, 49, 192, 195,
            49, 192, 195,
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
fn test_native_codegen_x86_ref_new_preserves_initial_value_in_bounded_heap() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-ref-new-bounded-heap-preserve",
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
        vec![73, 81, 73, 0, 0, 0, 0, 192],
        "x86_64 ref-new helper は bounded heap allocation 後も初期値を ref cell に保存する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_ref_new_uses_bounded_heap_cursor() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-ref-new-bounded-heap-cursor",
        r#"  (let [helper (emit-x86-selfhost-ref-new-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 0 (vector-length helper))
      0))"#,
    );

    assert!(
        !lines
            .windows(6)
            .any(|window| window == [9, 0, 0, 0, 15, 5]),
        "x86_64 ref-new helper は per-allocation mmap syscall を発行せず、bounded native heap cursor を使うべき"
    );
}

#[test]
fn test_native_codegen_x86_vector_new_preserves_capacity_in_bounded_heap() {
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
        vec![119, 81, 72, 137, 0, 0, 0, 137, 144, 144, 144, 144],
        "x86_64 vector-new helper は bounded heap allocation 後も vector header に capacity を保存する必要がある"
    );
}

#[test]
fn test_native_codegen_x86_vector_new_uses_bounded_heap_cursor() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-new-bounded-heap-cursor",
        r#"  (let [helper (emit-x86-selfhost-vector-new-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 0 (vector-length helper))
      0))"#,
    );

    assert!(
        !lines
            .windows(6)
            .any(|window| window == [9, 0, 0, 0, 15, 5]),
        "x86_64 vector-new helper は per-allocation mmap syscall を発行せず、bounded native heap cursor を使うべき"
    );
}

#[test]
fn test_native_codegen_x86_map_new_uses_bounded_heap_cursor() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-map-new-bounded-heap-cursor",
        r#"  (let [helper (emit-x86-selfhost-map-new-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 0 (vector-length helper))
      0))"#,
    );

    assert!(
        !lines
            .windows(6)
            .any(|window| window == [9, 0, 0, 0, 15, 5]),
        "x86_64 map-new helper は per-allocation mmap syscall を発行せず、bounded native heap cursor を使うべき"
    );
}

#[test]
fn test_native_codegen_x86_vector_push_growth_uses_bounded_heap_cursor() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-push-bounded-heap-growth",
        r#"  (let [helper (emit-x86-selfhost-vector-push-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 0 (vector-length helper))
      0))"#,
    );

    assert!(
        !lines
            .windows(6)
            .any(|window| window == [9, 0, 0, 0, 15, 5]),
        "x86_64 vector-push growth helper は per-growth mmap syscall を発行せず、bounded native heap cursor を使うべき"
    );
}

#[test]
fn test_native_codegen_x86_string_concat_uses_bounded_heap_cursor() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-string-concat-bounded-heap",
        r#"  (let [helper (emit-x86-selfhost-string-concat-helper)]
    (do
      (print (vector-length helper))
      (print-bytes-loop helper 0 (vector-length helper))
      0))"#,
    );

    assert!(
        !lines
            .windows(7)
            .any(|window| window == [184, 9, 0, 0, 0, 15, 5]),
        "x86_64 string-concat helper は per-allocation mmap syscall を発行せず、bounded native heap cursor を使うべき"
    );
}

#[test]
fn test_native_codegen_x86_int_to_string_uses_bounded_heap_cursor() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-int-to-string-bounded-heap",
        r#"  (let [helper (emit-x86-selfhost-int-to-string-helper)]
    (do
      (print-bytes-loop helper 0 (vector-length helper))
      0))"#,
    );

    assert!(
        !lines
            .windows(9)
            .any(|window| window == [72, 199, 192, 9, 0, 0, 0, 15, 5]),
        "x86_64 int-to-string helper は per-allocation mmap syscall を発行せず、bounded native heap cursor を使うべき"
    );
}

#[test]
fn test_native_codegen_x86_vector_new_call_targets_executable_entry_after_prefix() {
    let lines = run_x86_selfhost_runtime_helper_harness(
        "native-stage23-x86-vector-new-call-entry",
        r#"  (let [helper (emit-x86-selfhost-vector-new-helper)]
    (do
      (print (x86-helper-base-offset 4096 10))
      (print (x86-selfhost-vector-new-helper-offset 4096 10))
      (print (vector-length helper))
      (print (vector-get helper 2))
      (print (vector-get helper 3))
      (print (vector-get helper 4))
      (print (vector-get helper 5))
      (print (vector-get helper 6))
      (print (vector-get helper 7))
      (print (vector-get helper 8))
      (print (vector-get helper 9))
      0))"#,
    );

    assert_eq!(
        lines,
        vec![4097, 4547, 119, 137, 198, 72, 193, 230, 3, 72, 131],
        "x86 vector-new call は bounded heap helper の実行入口へ分岐するべき"
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
