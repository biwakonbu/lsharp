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
        "I32Const",
        "I32Add",
        "I32Mul",
        "I32WrapI64",
        "I64ExtendI32S",
        "I64ExtendI32U",
        "LocalGet",
        "LocalSet",
    ])
}

fn supported_native_opcodes_aarch64() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "I64Const",
        "I32Const",
        "I32Add",
        "I32Mul",
        "I32WrapI64",
        "I64ExtendI32S",
        "I64ExtendI32U",
        "LocalGet",
        "LocalSet",
    ])
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
    }
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
    let json = format!(
        "{{\n  \"entry_path\": \"{}\",\n  \"function_count\": {},\n  \"instruction_count\": {},\n  \"supported_x86_64\": [\"I64Const\", \"I64Add\", \"I64Sub\", \"I32Const\", \"I32Add\", \"I32Mul\", \"I32WrapI64\", \"I64ExtendI32S\", \"I64ExtendI32U\", \"LocalGet\", \"LocalSet\"],\n  \"supported_aarch64\": [\"I64Const\", \"I32Const\", \"I32Add\", \"I32Mul\", \"I32WrapI64\", \"I64ExtendI32S\", \"I64ExtendI32U\", \"LocalGet\", \"LocalSet\"],\n  \"unsupported_x86_64\": {},\n  \"unsupported_aarch64\": {},\n  \"opcode_histogram\": {}\n}}\n",
        json_escape(&report.entry_path),
        report.function_count,
        report.instruction_count,
        render_json_string_list(&report.unsupported_x86_64),
        render_json_string_list(&report.unsupported_aarch64),
        render_json_histogram(&report.opcode_histogram),
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
fn test_e2e_native_actual_stage23_gap_report_for_representative_entry() {
    let entry_path = selfhost_main_path();
    let compile_entry_path = entry_path.clone();
    let report = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        let module = lsharp_ir::compile_multi_file(&compile_entry_path)
            .expect("selfhost App/Main.ls の multi-file compile に失敗");
        collect_native_stage23_gap_report(&compile_entry_path, &module)
    });

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
        !report.unsupported_x86_64.is_empty(),
        "x86_64 native backend gap が空なのは不自然"
    );
    assert!(
        !report.unsupported_aarch64.is_empty(),
        "aarch64 native backend gap が空なのは不自然"
    );
    assert!(
        report.unsupported_x86_64.iter().any(|name| {
            name.starts_with("Call")
                || name.starts_with("If")
                || name.starts_with("Loop")
                || name.starts_with("Struct")
                || name.starts_with("I32")
        }),
        "x86_64 gap report は actual stage23 を塞いでいる代表 opcode を含むべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        report.unsupported_aarch64.iter().any(|name| {
            name.starts_with("Call")
                || name.starts_with("If")
                || name.starts_with("Loop")
                || name.starts_with("Struct")
                || name.starts_with("I32")
        }),
        "aarch64 gap report は actual stage23 を塞いでいる代表 opcode を含むべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report
            .unsupported_x86_64
            .iter()
            .any(|name| name == "LocalGet" || name == "LocalSet"),
        "x86_64 gap report から LocalGet/LocalSet は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report
            .unsupported_aarch64
            .iter()
            .any(|name| name == "LocalGet" || name == "LocalSet"),
        "aarch64 gap report から LocalGet/LocalSet は消えているべき: {:?}",
        report.unsupported_aarch64
    );
    assert!(
        !report.unsupported_x86_64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Const" | "I32Add" | "I32Mul" | "I32WrapI64" | "I64ExtendI32S" | "I64ExtendI32U"
            )
        }),
        "x86_64 gap report から i32 core opcode は消えているべき: {:?}",
        report.unsupported_x86_64
    );
    assert!(
        !report.unsupported_aarch64.iter().any(|name| {
            matches!(
                name.as_str(),
                "I32Const" | "I32Add" | "I32Mul" | "I32WrapI64" | "I64ExtendI32S" | "I64ExtendI32U"
            )
        }),
        "aarch64 gap report から i32 core opcode は消えているべき: {:?}",
        report.unsupported_aarch64
    );
}
