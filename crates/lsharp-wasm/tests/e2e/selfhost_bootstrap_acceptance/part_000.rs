use super::selfhost_bootstrap_four_layer::{
    BootstrapDiffArtifactFixture, bootstrap_diff_artifact_id,
    run_wasm_with_eleven_imports_compiler_mode, run_wasm_with_eleven_imports_compiler_mode_fs,
    write_bootstrap_diff_artifact,
};
use super::support::*;

fn run_bootstrap_acceptance_with_expanded_stack<T, F>(body: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, body)
}

// =============================================================================
// BOOT-04 受入テスト: True stage1-stage2-stage3 bootstrap の実体比較テスト
//
// acceptance criteria (phase11-implementation-plan.md BOOT-04 より):
//   test_e2e_bootstrap_stage1_stage2_match
//   test_e2e_bootstrap_fixed_point_stage2_stage3
//   test_e2e_bootstrap_stage1_section_stability
//   test_e2e_bootstrap_stage1_symbol_stability
// =============================================================================

// -----------------------------------------------------------------------------
// ローカルヘルパー: Wasm セクションパース
// -----------------------------------------------------------------------------

/// Wasm バイナリからセクション ID とサイズの列を抽出する
fn extract_sections(wasm: &[u8]) -> Vec<(u8, usize)> {
    let mut sections = Vec::new();
    let mut pos = 8; // magic(4) + version(4)
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        sections.push((section_id, size));
        pos += size;
    }
    sections
}

/// 指定セクション ID のバイト列を抽出する
fn extract_section_bytes(wasm: &[u8], target_id: u8) -> Option<Vec<u8>> {
    let mut pos = 8;
    while pos < wasm.len() {
        let section_id = wasm[pos];
        pos += 1;
        let mut size: usize = 0;
        let mut shift = 0;
        loop {
            if pos >= wasm.len() {
                break;
            }
            let byte = wasm[pos] as usize;
            pos += 1;
            size |= (byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        if section_id == target_id {
            return Some(wasm[pos..pos + size].to_vec());
        }
        pos += size;
    }
    None
}

/// 2つの Wasm バイト列が最初に食い違う位置を返す
fn first_diff_index(left: &[u8], right: &[u8]) -> Option<usize> {
    left.iter()
        .zip(right.iter())
        .position(|(a, b)| a != b)
        .or_else(|| {
            if left.len() == right.len() {
                None
            } else {
                Some(left.len().min(right.len()))
            }
        })
}

/// stage1 が stdout に出力した length-prefixed Wasm バイト列を復元する
fn parse_emitted_wasm_modules(output: &str, expected_modules: usize) -> Vec<Vec<u8>> {
    let values: Vec<usize> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.trim()
                .parse::<usize>()
                .unwrap_or_else(|_| panic!("数値でない stage1 出力: {line:?}"))
        })
        .collect();

    let mut pos = 0;
    let mut modules = Vec::with_capacity(expected_modules);
    for module_idx in 0..expected_modules {
        assert!(
            pos < values.len(),
            "module[{module_idx}] の長さ行が不足: {:?}",
            values
        );
        let len = values[pos];
        pos += 1;
        assert!(
            values.len() >= pos + len,
            "module[{module_idx}] の byte 数が不足: len={}, remaining={}",
            len,
            values.len().saturating_sub(pos)
        );
        let mut wasm = Vec::with_capacity(len);
        for &value in &values[pos..pos + len] {
            assert!(value <= u8::MAX as usize, "byte 値が範囲外: {value}");
            wasm.push(value as u8);
        }
        pos += len;
        modules.push(wasm);
    }
    assert_eq!(
        pos,
        values.len(),
        "想定外の trailing output が残っている: {:?}",
        &values[pos..]
    );
    modules
}

fn parse_printed_i64_lines(output: &str, context: &str) -> Vec<i64> {
    let mut values = Vec::new();
    let mut current = String::new();

    for ch in output.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
            continue;
        }
        if ch == '-' {
            if current.is_empty() {
                current.push(ch);
            } else {
                let value = current
                    .parse::<i64>()
                    .unwrap_or_else(|_| panic!("{context}: 数値でない debug 出力: {current:?}"));
                values.push(value);
                current.clear();
                current.push(ch);
            }
            continue;
        }
        if !current.is_empty() && current != "-" {
            let value = current
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("{context}: 数値でない debug 出力: {current:?}"));
            values.push(value);
        }
        current.clear();
    }

    if !current.is_empty() && current != "-" {
        let value = current
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("{context}: 数値でない debug 出力: {current:?}"));
        values.push(value);
    }

    assert!(
        !values.is_empty(),
        "{context}: 数値が見つからない debug 出力: {output:?}"
    );
    values
}

fn compiler_mode_probe_args(entry_path: &'static str, probe_arg_index: usize) -> Vec<&'static str> {
    assert!(
        probe_arg_index >= 2,
        "probe arg index は path より後である必要がある"
    );
    let mut args = vec![""; probe_arg_index + 1];
    args[0] = "compiler";
    args[1] = entry_path;
    args[probe_arg_index] = "1";
    args
}

fn run_stage1_compiler_probe(
    entry_path: &'static str,
    probe_arg_index: usize,
    context: &'static str,
) -> Vec<i64> {
    run_bootstrap_acceptance_with_expanded_stack(move || {
        let main_path = selfhost_main_path();
        let selfhost_root = main_path
            .parent()
            .expect("App/ ディレクトリ")
            .parent()
            .expect("src/ ディレクトリ")
            .parent()
            .expect("selfhost/ ルートディレクトリ")
            .to_path_buf();
        let stage1_wasm = compile_file_only(&main_path);
        assert_valid_wasm(&stage1_wasm);
        let args = compiler_mode_probe_args(entry_path, probe_arg_index);
        let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
            &stage1_wasm,
            Some(&selfhost_root),
            &args,
        )
        .unwrap_or_else(|err| panic!("{context}: stage1 probe 実行に失敗: {err}"));
        parse_printed_i64_lines(&output, context)
    })
}

fn run_stage1_main_compiler_probe(probe_arg_index: usize, context: &'static str) -> Vec<i64> {
    run_stage1_compiler_probe("src/App/Main.ls", probe_arg_index, context)
}

fn assert_probe_pattern(output: &[i64], pattern: &[Option<i64>], context: &str) {
    assert_eq!(
        output.len(),
        pattern.len(),
        "{context}: probe 出力長が不正: output={output:?}"
    );
    for (idx, expected) in pattern.iter().enumerate() {
        if let Some(expected) = expected {
            assert_eq!(
                output[idx], *expected,
                "{context}: probe marker mismatch at index {idx}: output={output:?}"
            );
        }
    }
}

fn assert_probe_pattern_in_order(output: &[i64], pattern: &[Option<i64>], context: &str) {
    let mut cursor = 0usize;
    for (pattern_idx, expected) in pattern.iter().enumerate() {
        match expected {
            Some(expected) => {
                let offset = output[cursor..]
                    .iter()
                    .position(|value| value == expected)
                    .unwrap_or_else(|| {
                        panic!(
                            "{context}: probe marker {expected} が見つからない: pattern_idx={pattern_idx}, output={output:?}"
                        )
                    });
                cursor += offset + 1;
            }
            None => {
                assert!(
                    cursor < output.len(),
                    "{context}: wildcard probe 値が不足: pattern_idx={pattern_idx}, output={output:?}"
                );
                cursor += 1;
            }
        }
    }
}

fn assert_probe_prefix(output: &[i64], pattern: &[Option<i64>], context: &str) {
    assert!(
        output.len() >= pattern.len(),
        "{context}: probe 出力が短すぎる: output={output:?}"
    );
    for (idx, expected) in pattern.iter().enumerate() {
        if let Some(expected) = expected {
            assert_eq!(
                output[idx], *expected,
                "{context}: probe prefix mismatch at index {idx}: output={output:?}"
            );
        }
    }
}

fn assert_probe_markers_in_order(output: &[i64], markers: &[i64], context: &str) {
    let mut next = 0usize;
    for value in output {
        if next < markers.len() && *value == markers[next] {
            next += 1;
        }
    }
    assert_eq!(
        next,
        markers.len(),
        "{context}: probe marker が不足: output={output:?}"
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_compile_phase_probe_reaches_compile_complete() {
    let output = run_stage1_main_compiler_probe(
        18,
        "stage1 compile-phase probe should finish Main.ls compile phase",
    );
    assert_probe_pattern(
        &output,
        &[
            Some(150),
            None,
            Some(151),
            None,
            Some(152),
            None,
            Some(153),
            None,
            Some(154),
            None,
        ],
        "stage1 compile-phase probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_build_phase_probe_reaches_build_complete() {
    let output =
        run_stage1_main_compiler_probe(14, "stage1 build-phase probe should finish Main.ls build");
    assert_probe_pattern_in_order(
        &output,
        &[
            Some(101),
            Some(102),
            None,
            Some(104),
            None,
            Some(50),
            None,
            Some(51),
            None,
            Some(52),
            None,
            Some(53),
            None,
            Some(54),
            None,
            Some(55),
            None,
            Some(56),
            None,
            Some(57),
            None,
            Some(58),
            None,
            Some(59),
            None,
            Some(60),
            None,
            Some(61),
            None,
            Some(62),
            None,
            Some(63),
            None,
            Some(64),
            None,
            Some(65),
            None,
            Some(66),
            None,
            Some(103),
            None,
        ],
        "stage1 build-phase probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_cache_probe_emits_cache_marker() {
    let output = run_stage1_main_compiler_probe(8, "stage1 cache probe should emit cache marker");
    assert_probe_pattern(&output, &[Some(80), None, None, None], "stage1 cache probe");
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_ast_chunked_step_progress_probe_reaches_first_pair_complete() {
    let output = run_stage1_main_compiler_probe(
        20,
        "stage1 ast chunked step progress probe should finish first pair",
    );
    assert_probe_pattern_in_order(
        &output,
        &[Some(150), None, Some(151), None, Some(153), None],
        "stage1 ast chunked step progress probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_module_resolver_first_defn_source_probe_reaches_prefix() {
    let output = run_stage1_compiler_probe(
        "src/App/ModuleResolver.ls",
        22,
        "stage1 ModuleResolver first-defn source probe should reach prefix",
    );
    assert_probe_prefix(
        &output,
        &[Some(301), None, Some(302), None],
        "stage1 ModuleResolver first-defn source probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_module_resolver_ast_chunked_step_probe_reaches_completion() {
    let output = run_stage1_compiler_probe(
        "src/App/ModuleResolver.ls",
        20,
        "stage1 ModuleResolver ast chunked step probe should finish file compile",
    );
    assert_probe_pattern_in_order(
        &output,
        &[Some(150), None, Some(151), None, Some(153), None],
        "stage1 ModuleResolver ast chunked step probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_backend_compiler_pair_progress_probe_reaches_final_markers() {
    let output = run_stage1_compiler_probe(
        "src/Backend/Wasm/Compiler.ls",
        19,
        "stage1 Backend/Wasm/Compiler pair progress probe should finish compile",
    );
    assert_probe_markers_in_order(
        &output,
        &[150, 151, 152, 160, 161, 153, 154],
        "stage1 Backend/Wasm/Compiler pair progress probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_compiler_mode_pair_progress_probe_reaches_final_markers() {
    let output = run_stage1_compiler_probe(
        "src/App/CompilerMode.ls",
        19,
        "stage1 App/CompilerMode pair progress probe should finish compile",
    );
    assert_probe_markers_in_order(
        &output,
        &[150, 151, 152, 160, 161, 153, 154],
        "stage1 App/CompilerMode pair progress probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_compiler_mode_token_debug_emits_token_count() {
    let output = run_stage1_compiler_probe(
        "src/App/CompilerMode.ls",
        7,
        "stage1 App/CompilerMode token debug should finish lexing",
    );
    assert!(
        output.len() >= 3 && output[0] == 72,
        "stage1 App/CompilerMode token debug: unexpected output={output:?}"
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_pipeline_smoke_pair_progress_probe_reaches_final_markers() {
    let output = run_stage1_compiler_probe(
        "src/App/PipelineSmoke.ls",
        19,
        "stage1 App/PipelineSmoke pair progress probe should finish compile",
    );
    assert_probe_markers_in_order(
        &output,
        &[150, 151, 152, 160, 161, 153, 154],
        "stage1 App/PipelineSmoke pair progress probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_native_codegen_pair_progress_probe_reaches_final_markers() {
    let output = run_stage1_compiler_probe(
        "src/Backend/Native/NativeCodegen.ls",
        19,
        "stage1 Backend/Native/NativeCodegen pair progress probe should finish compile",
    );
    assert_probe_markers_in_order(
        &output,
        &[150, 151, 152, 160, 161, 153, 154],
        "stage1 Backend/Native/NativeCodegen pair progress probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_native_codegen_cache_pairs_probe_emits_counts() {
    let output = run_stage1_compiler_probe(
        "src/Backend/Native/NativeCodegen.ls",
        9,
        "stage1 Backend/Native/NativeCodegen cache pairs probe should finish import loading",
    );
    assert_probe_prefix(
        &output,
        &[Some(81), None, None, None],
        "stage1 Backend/Native/NativeCodegen cache pairs probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_native_codegen_cache_probe_emits_marker() {
    let output = run_stage1_compiler_probe(
        "src/Backend/Native/NativeCodegen.ls",
        8,
        "stage1 Backend/Native/NativeCodegen cache probe should finish entry parse",
    );
    assert_probe_prefix(
        &output,
        &[Some(80), None, None, None],
        "stage1 Backend/Native/NativeCodegen cache probe",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_native_codegen_ir_debug_emits_decl_count() {
    let output = run_stage1_compiler_probe(
        "src/Backend/Native/NativeCodegen.ls",
        6,
        "stage1 Backend/Native/NativeCodegen ir debug should finish parse",
    );
    assert_probe_prefix(
        &output,
        &[Some(71), None, None],
        "stage1 Backend/Native/NativeCodegen ir debug",
    );
}

#[test]
#[ignore]
fn test_e2e_bootstrap_stage1_native_codegen_token_debug_emits_token_count() {
    let output = run_stage1_compiler_probe(
        "src/Backend/Native/NativeCodegen.ls",
        7,
        "stage1 Backend/Native/NativeCodegen token debug should finish lexing",
    );
    assert!(
        output.len() >= 3 && output[0] == 72,
        "stage1 Backend/Native/NativeCodegen token debug: unexpected output={output:?}"
    );
}

/// selfhost runtime 10-import layout の Wasm モジュールを instantiate して i64 export を呼び出す
fn run_exported_i64_with_runtime_imports(wasm: &[u8], export_name: &str) -> i64 {
    let engine = wasmtime::Engine::default();
    let module = wasmtime::Module::new(&engine, wasm)
        .expect("runtime 10-import 付き stage2 Wasm の Module 構築に失敗");

    struct State {
        next_alloc: i64,
        root_stack: Vec<i64>,
    }

    let mut store = wasmtime::Store::new(
        &engine,
        State {
            next_alloc: 1024,
            root_stack: Vec::new(),
        },
    );
    let alloc = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>, size: i64| -> i64 {
            let base = caller.data().next_alloc;
            caller.data_mut().next_alloc = base + size;
            base
        },
    );
    let print = wasmtime::Func::wrap(&mut store, |_: wasmtime::Caller<'_, State>, _: i64| {});
    let read_file = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64| -> i64 { 0 },
    );
    let command_line_arg = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64| -> i64 { 0 },
    );
    let string_concat = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64, _: i64| -> i64 { 0 },
    );
    let substring = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64, _: i64, _: i64| -> i64 { 0 },
    );
    let file_exists = wasmtime::Func::wrap(
        &mut store,
        |_: wasmtime::Caller<'_, State>, _: i64| -> i64 { 0 },
    );
    let root_push = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>, value: i64| -> i64 {
            let slot =
                i64::try_from(caller.data().root_stack.len()).expect("root_push: slot overflow");
            caller.data_mut().root_stack.push(value);
            slot
        },
    );
    let root_pop = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>| -> i64 {
            caller.data_mut().root_stack.pop().unwrap_or(0)
        },
    );
    let root_set = wasmtime::Func::wrap(
        &mut store,
        |mut caller: wasmtime::Caller<'_, State>, slot: i64, value: i64| -> i64 {
            let idx = usize::try_from(slot).expect("root_set: slot must be non-negative");
            if idx < caller.data().root_stack.len() {
                caller.data_mut().root_stack[idx] = value;
            }
            slot
        },
    );
    let instance = wasmtime::Instance::new(
        &mut store,
        &module,
        &[
            alloc.into(),
            print.into(),
            read_file.into(),
            command_line_arg.into(),
            string_concat.into(),
            substring.into(),
            file_exists.into(),
            root_push.into(),
            root_pop.into(),
            root_set.into(),
        ],
    )
    .expect("runtime 10-import 付き stage2 Wasm のインスタンス化に失敗");
    let func = instance
        .get_typed_func::<(), i64>(&mut store, export_name)
        .unwrap_or_else(|e| panic!("{export_name} export の取得に失敗: {e}"));
    func.call(&mut store, ())
        .expect("runtime 10-import 付き stage2 Wasm export の呼び出しに失敗")
}

/// selfhost runtime 10-import layout 用 bootstrap ハーネス:
/// compile-program-functions-with-base 経由で stage2 を生成し print する
fn build_simple_bootstrap_harness(stage2_src: &str) -> String {
    format!(
        r#"
(defn bootstrap-append-bytes [dst src idx count]
  (if (>= idx count)
    dst
    (bootstrap-append-bytes
      (vector-push dst (vector-get src idx))
      src (+ idx 1) count)))

(defn bootstrap-build-stage2 [src]
  (let [program (parse-program src)
        pair    (compile-program-functions-with-base program 10)
        functions   (vector-get pair 1)
        func-count  (vector-length functions)
        header      (emit-header)
        type-sec    (emit-type-section-wasi-quad-functions functions)
        import-sec  (emit-import-section-runtime)
        func-sec    (emit-function-section-wasi-quad-functions functions)
        memory-sec  (emit-memory-section)
        export-sec  (emit-export-section-main-index (+ 9 func-count))
        code-sec    (emit-code-section-wasi-quad-functions functions)
        b0 (bootstrap-append-bytes (vector-new 64) header    0 (vector-length header))
        b1 (bootstrap-append-bytes b0 type-sec    0 (vector-length type-sec))
        b2 (bootstrap-append-bytes b1 import-sec  0 (vector-length import-sec))
        b3 (bootstrap-append-bytes b2 func-sec    0 (vector-length func-sec))
        b4 (bootstrap-append-bytes b3 memory-sec  0 (vector-length memory-sec))
        b5 (bootstrap-append-bytes b4 export-sec  0 (vector-length export-sec))]
    (bootstrap-append-bytes b5 code-sec 0 (vector-length code-sec))))

(defn bootstrap-print-bytes [bytes idx count]
  (if (>= idx count) 0
    (do (print (vector-get bytes idx))
        (bootstrap-print-bytes bytes (+ idx 1) count))))

(defn bootstrap-print-module [bytes]
  (let [count (vector-length bytes)]
    (do (print count) (bootstrap-print-bytes bytes 0 count) 0)))

(defn main []
  (let [stage2 (bootstrap-build-stage2 "{}")]
    (do (bootstrap-print-module stage2) 0)))
"#,
        stage2_src.replace('"', "\\\"")
    )
}
