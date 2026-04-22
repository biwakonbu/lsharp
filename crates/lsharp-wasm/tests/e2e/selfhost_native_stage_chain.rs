use super::support::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NATIVE_STAGE_CHAIN_COUNTER: AtomicUsize = AtomicUsize::new(0);
static NATIVE_HOST_EXEC_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeTargetSummary {
    native_len: i64,
    object_len: i64,
    link_response_len: i64,
    target_arch: i64,
    target_format: i64,
    linker_kind: i64,
    ir_len: i64,
    object_byte0: i64,
    object_byte4: i64,
    response_output_byte: i64,
    response_object_byte: i64,
    multi_response_object2_byte: i64,
    multi_link_response_len: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeBundleSummary {
    program_object_hash: i64,
    runtime_object_hash: i64,
    response_path_hash: i64,
    program_binary_hash: i64,
    response_text_hash: i64,
    response_text_len: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeObservationSummary {
    target: NativeTargetSummary,
    bundle: NativeBundleSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeStageObservation {
    darwin: NativeObservationSummary,
    linux: NativeObservationSummary,
    aarch64: NativeObservationSummary,
}

#[derive(Debug)]
struct NativeHostArtifactBundle {
    program_object: Vec<u8>,
    runtime_object: Vec<u8>,
    response_text: String,
    program_binary: Vec<u8>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeHostExecutionResult {
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeHostBundleObservation {
    program_object_hash: u64,
    runtime_object_hash: u64,
    response_text_hash: u64,
    program_binary_hash: u64,
    stdout_hash: u64,
    stderr_hash: u64,
    exit_code: i32,
}

#[derive(Debug, Clone)]
struct NativeEntrypointBundle {
    function_start_len: usize,
    main_func_idx: usize,
    declared_code_len: usize,
    entrypoint_offset: usize,
    code_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeHostArtifactObservation {
    program_object_hash: u64,
    runtime_object_hash: u64,
    response_text_hash: u64,
    program_binary_hash: u64,
}

/// NATIVE-05/NATIVE-06: selfhost/src/App/Main.ls の native summary が
/// direct native pipeline harness と一致すること。
#[test]
fn test_e2e_selfhost_main_native_summary_matches_direct_pipeline_harness() {
    let main_output = compile_and_run_file(&selfhost_main_path());
    let main_lines = parse_numeric_lines(&main_output);
    assert!(
        main_lines.len() >= 71,
        "selfhost/src/App/Main.ls native summary 出力が不足: {:?}",
        main_lines
    );

    let direct_output = run_native_pipeline_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import Backend.Native.NativeEmit)
(import Backend.Native.Linker)

(defn make-instr [opcode operand]
  (vector-push (vector-push (vector-new 2) opcode) operand))

(defn emit-summary [triple-id]
  (let [instr (make-instr 1 42)
        ir (vector-push (vector-new 1) instr)
        target (make-target triple-id)
        native (emit-native ir target)
        object (emit-object native target)
        single-objects (vector-push (vector-new 1) (vector-length object))
        single-args (build-linker-args single-objects 99 target)
        single-response (generate-response-file single-args)
        multi-objects (vector-push (vector-push (vector-new 2) (vector-length object)) (vector-length object))
        multi-args (build-linker-args multi-objects 99 target)
        multi-response (generate-response-file multi-args)]
    (do
      (print (vector-length native))
      (print (vector-length object))
      (print (link-objects single-objects 99 target))
      (print (target-arch target))
      (print (target-obj-format target))
      (print (select-linker target))
      (print (vector-length ir))
      (print (vector-get object 0))
      (print (vector-get object 4))
      (print (vector-get single-response 2))
      (print (vector-get single-response 4))
      (print (vector-get multi-response 6))
      (print (link-objects multi-objects 99 target)))))

(defn main []
  (do
    (emit-summary 1)
    (emit-summary 3)
    (emit-summary 2)
    0))"#,
    );
    let direct_lines = parse_numeric_lines(&direct_output);
    assert!(
        direct_lines.len() >= 39,
        "direct native pipeline harness 出力が不足: {:?}",
        direct_lines
    );

    let expected = parse_direct_summaries(&direct_lines);
    let actual = [
        parse_main_summary(&main_lines, NativeSummaryKind::Darwin),
        parse_main_summary(&main_lines, NativeSummaryKind::Linux),
        parse_main_summary(&main_lines, NativeSummaryKind::Aarch64),
    ];

    assert_eq!(
        actual, expected,
        "selfhost/src/App/Main.ls native summary が direct native pipeline harness と一致しない"
    );
}

/// V2-08: selfhost main smoke が canonical native bundle summary を公開すること。
#[test]
fn test_e2e_selfhost_main_native_bundle_summary_matches_canonical_contract() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines = parse_numeric_lines(&output);
    assert!(
        lines.len() >= 89,
        "selfhost/src/App/Main.ls native bundle summary 出力が不足: {:?}",
        lines
    );

    let expected = expected_native_bundle_summary();
    let actual = [
        parse_main_bundle_summary(&lines, NativeBundleKind::Darwin),
        parse_main_bundle_summary(&lines, NativeBundleKind::Linux),
        parse_main_bundle_summary(&lines, NativeBundleKind::Aarch64),
    ];

    assert_eq!(
        actual,
        [expected.clone(), expected.clone(), expected],
        "selfhost/src/App/Main.ls native bundle summary が canonical contract と一致しない"
    );
}

/// V2-08: stage1-native の比較面として使う observation summary が 2 回実行で一致すること。
#[test]
fn test_e2e_stage1_native_observation_summary_two_run_determinism() {
    let run1 = compile_and_run_file(&selfhost_main_path());
    let run2 = compile_and_run_file(&selfhost_main_path());

    let obs1 = parse_main_stage_observation(&parse_numeric_lines(&run1));
    let obs2 = parse_main_stage_observation(&parse_numeric_lines(&run2));

    assert_eq!(
        obs1, obs2,
        "stage1-native observation summary が 2 回実行で一致しない"
    );
}

fn parse_numeric_lines(output: &str) -> Vec<i64> {
    output
        .trim()
        .lines()
        .map(|line| {
            line.parse::<i64>()
                .unwrap_or_else(|_| panic!("numeric line ではない出力を検出: {line}"))
        })
        .collect()
}

fn parse_direct_summaries(lines: &[i64]) -> [NativeTargetSummary; 3] {
    [
        parse_direct_summary(lines, 0),
        parse_direct_summary(lines, 13),
        parse_direct_summary(lines, 26),
    ]
}

fn parse_direct_summary(lines: &[i64], offset: usize) -> NativeTargetSummary {
    NativeTargetSummary {
        native_len: lines[offset],
        object_len: lines[offset + 1],
        link_response_len: lines[offset + 2],
        target_arch: lines[offset + 3],
        target_format: lines[offset + 4],
        linker_kind: lines[offset + 5],
        ir_len: lines[offset + 6],
        object_byte0: lines[offset + 7],
        object_byte4: lines[offset + 8],
        response_output_byte: lines[offset + 9],
        response_object_byte: lines[offset + 10],
        multi_response_object2_byte: lines[offset + 11],
        multi_link_response_len: lines[offset + 12],
    }
}

enum NativeSummaryKind {
    Darwin,
    Linux,
    Aarch64,
}

fn parse_main_summary(lines: &[i64], kind: NativeSummaryKind) -> NativeTargetSummary {
    match kind {
        NativeSummaryKind::Darwin => NativeTargetSummary {
            native_len: lines[32],
            object_len: lines[33],
            link_response_len: lines[34],
            target_arch: lines[35],
            target_format: lines[36],
            linker_kind: lines[37],
            ir_len: lines[38],
            object_byte0: lines[56],
            object_byte4: lines[57],
            response_output_byte: lines[62],
            response_object_byte: lines[63],
            multi_response_object2_byte: lines[68],
            multi_link_response_len: lines[46],
        },
        NativeSummaryKind::Linux => NativeTargetSummary {
            native_len: lines[39],
            object_len: lines[40],
            link_response_len: lines[41],
            target_arch: lines[42],
            target_format: lines[43],
            linker_kind: lines[44],
            ir_len: lines[45],
            object_byte0: lines[58],
            object_byte4: lines[59],
            response_output_byte: lines[64],
            response_object_byte: lines[65],
            multi_response_object2_byte: lines[69],
            multi_link_response_len: lines[47],
        },
        NativeSummaryKind::Aarch64 => NativeTargetSummary {
            native_len: lines[48],
            object_len: lines[49],
            link_response_len: lines[50],
            target_arch: lines[51],
            target_format: lines[52],
            linker_kind: lines[53],
            ir_len: lines[54],
            object_byte0: lines[60],
            object_byte4: lines[61],
            response_output_byte: lines[66],
            response_object_byte: lines[67],
            multi_response_object2_byte: lines[70],
            multi_link_response_len: lines[55],
        },
    }
}

fn parse_main_stage_observation(lines: &[i64]) -> NativeStageObservation {
    NativeStageObservation {
        darwin: NativeObservationSummary {
            target: parse_main_summary(lines, NativeSummaryKind::Darwin),
            bundle: parse_main_bundle_summary(lines, NativeBundleKind::Darwin),
        },
        linux: NativeObservationSummary {
            target: parse_main_summary(lines, NativeSummaryKind::Linux),
            bundle: parse_main_bundle_summary(lines, NativeBundleKind::Linux),
        },
        aarch64: NativeObservationSummary {
            target: parse_main_summary(lines, NativeSummaryKind::Aarch64),
            bundle: parse_main_bundle_summary(lines, NativeBundleKind::Aarch64),
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum NativeBundleKind {
    Darwin,
    Linux,
    Aarch64,
}

fn parse_main_bundle_summary(lines: &[i64], kind: NativeBundleKind) -> NativeBundleSummary {
    let offset = match kind {
        NativeBundleKind::Darwin => 71,
        NativeBundleKind::Linux => 77,
        NativeBundleKind::Aarch64 => 83,
    };
    NativeBundleSummary {
        program_object_hash: lines[offset],
        runtime_object_hash: lines[offset + 1],
        response_path_hash: lines[offset + 2],
        program_binary_hash: lines[offset + 3],
        response_text_hash: lines[offset + 4],
        response_text_len: lines[offset + 5],
    }
}

fn expected_native_bundle_summary() -> NativeBundleSummary {
    let program_object = "program.o";
    let runtime_object = "runtime.o";
    let response_path = "linker-response.txt";
    let program_binary = "program.native";
    let response_text = "-o\nprogram.native\nprogram.o\nruntime.o\n";

    NativeBundleSummary {
        program_object_hash: lsharp_name_hash(program_object),
        runtime_object_hash: lsharp_name_hash(runtime_object),
        response_path_hash: lsharp_name_hash(response_path),
        program_binary_hash: lsharp_name_hash(program_binary),
        response_text_hash: lsharp_name_hash(response_text),
        response_text_len: response_text.chars().count() as i64,
    }
}

fn lsharp_name_hash(text: &str) -> i64 {
    text.chars().fold(0_i64, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(i64::from(u32::from(ch)))
    })
}

fn observe_native_host_bundle(bundle: &NativeHostArtifactBundle) -> NativeHostBundleObservation {
    let artifact = observe_native_host_artifact_bundle(bundle);
    NativeHostBundleObservation {
        program_object_hash: artifact.program_object_hash,
        runtime_object_hash: artifact.runtime_object_hash,
        response_text_hash: artifact.response_text_hash,
        program_binary_hash: artifact.program_binary_hash,
        stdout_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(&bundle.stdout),
        stderr_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(&bundle.stderr),
        exit_code: bundle.exit_code,
    }
}

fn observe_native_host_artifact_bundle(
    bundle: &NativeHostArtifactBundle,
) -> NativeHostArtifactObservation {
    NativeHostArtifactObservation {
        program_object_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(
            &bundle.program_object,
        ),
        runtime_object_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(
            &bundle.runtime_object,
        ),
        response_text_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(
            bundle.response_text.as_bytes(),
        ),
        program_binary_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(
            &bundle.program_binary,
        ),
    }
}

fn host_native_exec_supported() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

fn write_native_host_bundle_artifact(
    root_dir: &std::path::Path,
    label: &str,
    bundle: &NativeHostArtifactBundle,
) -> Result<(), String> {
    let stage_dir = root_dir.join(label);
    let _ = std::fs::remove_dir_all(&stage_dir);
    std::fs::create_dir_all(&stage_dir)
        .map_err(|e| format!("native proxy artifact dir 作成失敗: {e}"))?;

    std::fs::write(stage_dir.join("program.o"), &bundle.program_object)
        .map_err(|e| format!("program.o 書き込み失敗: {e}"))?;
    std::fs::write(stage_dir.join("runtime.o"), &bundle.runtime_object)
        .map_err(|e| format!("runtime.o 書き込み失敗: {e}"))?;
    std::fs::write(stage_dir.join("linker-response.txt"), &bundle.response_text)
        .map_err(|e| format!("linker-response.txt 書き込み失敗: {e}"))?;
    let program_binary_path = stage_dir.join("program.native");
    std::fs::write(&program_binary_path, &bundle.program_binary)
        .map_err(|e| format!("program.native 書き込み失敗: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let permissions = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&program_binary_path, permissions)
            .map_err(|e| format!("program.native の execute bit 設定失敗: {e}"))?;
    }
    std::fs::write(stage_dir.join("stdout.txt"), &bundle.stdout)
        .map_err(|e| format!("stdout.txt 書き込み失敗: {e}"))?;
    std::fs::write(stage_dir.join("stderr.txt"), &bundle.stderr)
        .map_err(|e| format!("stderr.txt 書き込み失敗: {e}"))?;

    let observation = observe_native_host_bundle(bundle);
    let summary = format!(
        "{{\"label\":\"{label}\",\"exit_code\":{},\"program_object_hash\":{},\"runtime_object_hash\":{},\"response_text_hash\":{},\"program_binary_hash\":{},\"stdout_hash\":{},\"stderr_hash\":{}}}",
        observation.exit_code,
        observation.program_object_hash,
        observation.runtime_object_hash,
        observation.response_text_hash,
        observation.program_binary_hash,
        observation.stdout_hash,
        observation.stderr_hash,
    );
    std::fs::write(stage_dir.join("summary.json"), summary)
        .map_err(|e| format!("summary.json 書き込み失敗: {e}"))?;
    Ok(())
}

fn maybe_write_native_host_bundle_artifact(
    label: &str,
    bundle: &NativeHostArtifactBundle,
) -> Result<(), String> {
    let Some(root_dir) = std::env::var_os("LSHARP_NATIVE_PROXY_ARTIFACT_DIR") else {
        return Ok(());
    };
    write_native_host_bundle_artifact(&std::path::PathBuf::from(root_dir), label, bundle)
}

/// NATIVE-02: NativeTarget descriptor が policy field を公開すること。
///
/// target descriptor を単なる triple から一段拡張し、calling convention /
/// stack alignment / section policy / relocation call policy /
/// response file style / runtime object kind を取得できることを固定する。
#[test]
fn test_e2e_native_target_descriptor_exposes_policy_fields() {
    let output = run_native_pipeline_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)

(defn emit-target [triple-id]
  (let [target (make-target triple-id)]
    (do
      (print (vector-length target))
      (print (target-arch target))
      (print (target-calling-convention target))
      (print (target-stack-alignment target))
      (print (target-section-policy target))
      (print (target-reloc-call target))
      (print (target-linker-flavor target))
      (print (target-response-file-style target))
      (print (target-runtime-policy target))
      (print (target-runtime-object-kind target))
      0)))

(defn main []
  (do
    (emit-target 1)
    (emit-target 3)
    (emit-target 2)
    0))"#,
    );

    let lines = parse_numeric_lines(&output);
    assert_eq!(
        lines,
        vec![
            12, 1, 1, 16, 1, 1, 1, 1, 1, 1, // x86_64-apple-darwin
            12, 1, 1, 16, 2, 1, 2, 1, 1, 1, // x86_64-unknown-linux-gnu
            12, 2, 2, 16, 1, 2, 1, 1, 1, 1, // aarch64-apple-darwin
        ],
        "NativeTarget descriptor policy field が期待値と一致しない"
    );
}

/// V2-08: representative build entry で使う native artifact 名が canonical に固定されること。
#[test]
fn test_e2e_native_linker_exposes_canonical_stage_artifact_paths() {
    let output = run_native_pipeline_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.Linker)

(defn emit-artifacts [triple-id]
  (let [target (make-target triple-id)]
    (do
      (print-string (default-program-object-path target))
      (print-string "\n")
      (print-string (default-runtime-object-path target))
      (print-string "\n")
      (print-string (default-linker-response-path target))
      (print-string "\n")
      (print-string (default-program-binary-path target))
      (print-string "\n")
      0)))

(defn main []
  (do
    (emit-artifacts 1)
    (emit-artifacts 3)
    (emit-artifacts 2)
    0))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "program.o",
            "runtime.o",
            "linker-response.txt",
            "program.native",
            "program.o",
            "runtime.o",
            "linker-response.txt",
            "program.native",
            "program.o",
            "runtime.o",
            "linker-response.txt",
            "program.native",
        ],
        "native linker artifact contract が canonical 名からずれている"
    );
}

/// V2-08: native linker の response file テキストが canonical 順序で構築されること。
#[test]
fn test_e2e_native_linker_generates_canonical_response_file_text() {
    let output = run_native_pipeline_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.Linker)

(defn emit-response [triple-id]
  (let [target (make-target triple-id)
    objects (vector-push
              (vector-push (vector-new 2) (default-program-object-path target))
              (default-runtime-object-path target))
    args (build-linker-response-args objects (default-program-binary-path target) target)
    response (generate-response-file-text args)]
    (do
      (print-string response)
      0)))

(defn main []
  (do
    (emit-response 1)
    (emit-response 3)
    (emit-response 2)
    0))"#,
    );

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            "-o",
            "program.native",
            "program.o",
            "runtime.o",
            "-o",
            "program.native",
            "program.o",
            "runtime.o",
            "-o",
            "program.native",
            "program.o",
            "runtime.o",
        ],
        "native linker response file text が canonical 順序からずれている"
    );
}

/// NATIVE-05: stage1-native 二回実行の決定性 (stage1→stage2 等価の前提証明)
///
/// `selfhost/src/App/Main.ls` を二度独立にコンパイル・実行し、全出力行が一致することを確認する。
/// これは「stage0 が生成する stage1 は決定的であり、stage1 が生成する stage2 と一致する」
/// というブートストラップ等価性の基盤となる証拠である。
#[test]
fn test_e2e_stage1_native_two_run_determinism() {
    let path = selfhost_main_path();

    let run1 = compile_and_run_file(&path);
    let run2 = compile_and_run_file(&path);

    let lines1: Vec<&str> = run1.trim().lines().collect();
    let lines2: Vec<&str> = run2.trim().lines().collect();

    assert!(
        lines1.len() >= 71,
        "run1: selfhost/src/App/Main.ls 出力行数が不足 (got {}): {:?}",
        lines1.len(),
        &lines1[..lines1.len().min(10)]
    );
    assert_eq!(
        lines1.len(),
        lines2.len(),
        "二回の実行で出力行数が異なる: run1={}, run2={}",
        lines1.len(),
        lines2.len()
    );

    // 全行が一致すること (native 関連行 32+ を含む全出力)
    for (i, (l1, l2)) in lines1.iter().zip(lines2.iter()).enumerate() {
        assert_eq!(
            l1, l2,
            "行 {i} が二回の実行で異なる: run1={l1:?}, run2={l2:?} \
             — stage1 決定性が損なわれている"
        );
    }

    // native 関連行 (32以降) を個別確認: darwin native_len, linux native_len, aarch64 native_len
    let parse_line = |lines: &[&str], i: usize| -> i64 {
        lines[i]
            .parse::<i64>()
            .unwrap_or_else(|_| panic!("行 {i} が数値でない: {:?}", lines[i]))
    };
    let darwin_native_len_r1 = parse_line(&lines1, 32);
    let linux_native_len_r1 = parse_line(&lines1, 39);
    let aarch64_native_len_r1 = parse_line(&lines1, 48);

    let darwin_native_len_r2 = parse_line(&lines2, 32);
    let linux_native_len_r2 = parse_line(&lines2, 39);
    let aarch64_native_len_r2 = parse_line(&lines2, 48);

    assert_eq!(
        darwin_native_len_r1, darwin_native_len_r2,
        "darwin native_len が二回で異なる"
    );
    assert_eq!(
        linux_native_len_r1, linux_native_len_r2,
        "linux native_len が二回で異なる"
    );
    assert_eq!(
        aarch64_native_len_r1, aarch64_native_len_r2,
        "aarch64 native_len が二回で異なる"
    );

    // native_len が 0 でないこと (実際にネイティブコードが生成されている)
    assert!(
        darwin_native_len_r1 > 0,
        "darwin native_len が 0 — native コード生成が失敗している"
    );
    assert!(
        linux_native_len_r1 > 0,
        "linux native_len が 0 — native コード生成が失敗している"
    );
    assert!(
        aarch64_native_len_r1 > 0,
        "aarch64 native_len が 0 — native コード生成が失敗している"
    );
}

/// V2-08: representative build entry を actual import-count 付き function-meta bundle へ落とせること。
#[test]
fn test_e2e_selfhost_main_native_function_meta_bundle_with_import_count_emits_code_bytes() {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "native-compiler-runtime",
        &[
            "Main.ls",
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "Parser.ls",
            "IR.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "CompilerMode.ls",
            "Compiler.ls",
            "WasiBackend.ls",
            "WasmEmit.ls",
            "ModuleResolver.ls",
            "CompilerMode.ls",
            "NativeTarget.ls",
            "NativeCodegen.ls",
            "NativeEmit.ls",
        ],
        "src/App/HarnessMain.ls",
        r#"(module App.HarnessMain)
(import App.CompilerMode)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn push-import-placeholders [idx count result]
  (if (>= idx count)
    result
    (push-import-placeholders
      (+ idx 1)
      count
      (vector-push result (make-function-meta 0 0 (vector-new 0))))))

(defn append-vector-loop [dst src idx len]
  (if (>= idx len)
    dst
    (append-vector-loop
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      len)))

(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        payload (compile-file-functions-payload-with-cache "src/App/Main.ls" 10 cache-ref parse-count-ref)
        functions (vector-get payload 0)
        callables (append-vector-loop (push-import-placeholders 0 10 (vector-new 32)) functions 0 (vector-length functions))
        native-callables (normalize-selfhost-native-function-metas callables)
        target (host-target)
        code (emit-native-function-meta-bundle-with-import-count native-callables 10 target)]
    (do
      (print (vector-length callables))
      (print (vector-length code))
      0)))"#,
        &[],
    )
    .expect("representative native function-meta bundle harness 実行に失敗");

    let lines = parse_numeric_lines(&output);
    assert!(
        lines.len() >= 2,
        "representative native bundle summary 出力が不足: {lines:?}"
    );
    assert!(
        lines[0] > 0,
        "representative entry の function-meta 数が 0: {lines:?}"
    );
    assert!(
        lines[1] > 0,
        "representative entry の native bundle size が 0: {lines:?}"
    );
}

fn run_native_pipeline_harness(entry_source: &str) -> String {
    let id = NATIVE_STAGE_CHAIN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-stage-chain", id);
    std::fs::create_dir_all(&dir).expect("native stage-chain fixture dir 作成失敗");
    let entry_source = entry_source.to_string();
    let work_dir = dir.clone();

    let result = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        for name in [
            "IR.ls",
            "NativeTarget.ls",
            "NativeCodegen.ls",
            "NativeEmit.ls",
            "Linker.ls",
        ] {
            let path = work_dir.join(selfhost_fixture_module_relative_path(name));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("native stage-chain parent dir 作成失敗");
            }
            std::fs::write(&path, selfhost_module(name))
                .unwrap_or_else(|_| panic!("{name} 書き込み失敗"));
        }
        std::fs::write(work_dir.join("Main.ls"), entry_source).expect("Main.ls 書き込み失敗");
        compile_and_run_file(&work_dir.join("Main.ls"))
    });

    let _ = std::fs::remove_dir_all(&dir);
    result
}

// =============================================================================
// NATIVE-HOST-01: stage1-native-emitted object をホストでリンク・実行する
// =============================================================================

/// stage1-native パイプラインでホスト target 向けの生コードバイト列を取得するハーネス
///
/// L# セルフホストパイプライン (Wasm 経由) で `emit-native` を実行し、
/// 生成したネイティブ機械語バイトを Vec<u8> として返す。
/// バイトは1行1数値 (0-255) として stdout に出力される。
fn run_native_codegen_host_bytes_harness(entry_source: &str) -> Vec<u8> {
    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-host-bytes", id);
    std::fs::create_dir_all(&dir).expect("native host-bytes fixture dir 作成失敗");
    let entry_source = entry_source.to_string();
    let work_dir = dir.clone();

    let result = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        for name in ["IR.ls", "NativeTarget.ls", "NativeCodegen.ls"] {
            let path = work_dir.join(selfhost_fixture_module_relative_path(name));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("native host-bytes parent dir 作成失敗");
            }
            std::fs::write(&path, selfhost_module(name))
                .unwrap_or_else(|_| panic!("{name} 書き込み失敗"));
        }
        std::fs::write(work_dir.join("Main.ls"), entry_source).expect("Main.ls 書き込み失敗");
        let output = compile_and_run_file(&work_dir.join("Main.ls"));
        output
            .trim()
            .lines()
            .map(|line| {
                line.parse::<u8>()
                    .unwrap_or_else(|_| panic!("byte parse 失敗: {line}"))
            })
            .collect::<Vec<u8>>()
    });

    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn run_selfhost_main_native_function_meta_bundle_host_bytes_harness() -> NativeEntrypointBundle {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "native-stage23-representative-host-bytes",
        &[
            "Main.ls",
            "Token.ls",
            "AST.ls",
            "Lexer.ls",
            "Parser.ls",
            "IR.ls",
            "Type.ls",
            "TypeScheme.ls",
            "TypeInferCore.ls",
            "TypeInferFunctions.ls",
            "TypeInferBuiltins.ls",
            "TypeInfer.ls",
            "TypeInferApply.ls",
            "TypeInferBlock.ls",
            "TypeInferPattern.ls",
            "TypeInferRecord.ls",
            "CompilerMode.ls",
            "Compiler.ls",
            "WasiBackend.ls",
            "WasmEmit.ls",
            "ModuleResolver.ls",
            "CompilerMode.ls",
            "NativeTarget.ls",
            "NativeCodegen.ls",
            "NativeEmit.ls",
        ],
        "src/App/HarnessMain.ls",
        r#"(module App.HarnessMain)
(import App.CompilerMode)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn push-import-placeholders [idx count result]
  (if (>= idx count)
    result
    (push-import-placeholders
      (+ idx 1)
      count
      (vector-push result (make-function-meta 0 0 (vector-new 0))))))

(defn append-vector-loop [dst src idx len]
  (if (>= idx len)
    dst
    (append-vector-loop
      (vector-push dst (vector-get src idx))
      src
      (+ idx 1)
      len)))

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pairs (compile-file-pairs-with-cache "src/App/Main.ls" cache-ref parse-count-ref)
        reg-result (register-all-pairs pairs 0 (vector-length pairs) (map-new) 10)
        ftable (vector-get reg-result 0)
        main-pair (vector-get pairs (- (vector-length pairs) 1))
        main-decls (vector-get main-pair 1)
        main-defn-idx (find-first-defn-index main-decls 0 (vector-length main-decls))
        main-hash (vector-get (vector-get main-decls main-defn-idx) 1)
        main-func-idx (map-get ftable main-hash)
        payload (compile-file-functions-payload-with-cache "src/App/Main.ls" 10 cache-ref parse-count-ref)
        functions (vector-get payload 0)
        callables (append-vector-loop (push-import-placeholders 0 10 (vector-new 32)) functions 0 (vector-length functions))
        target (host-target)
        bundle-payload (emit-native-selfhost-function-meta-bundle-entrypoint-payload-for-function-with-import-count callables 10 main-func-idx target)
        code (vector-get bundle-payload 0)
        entrypoint-offset (vector-get bundle-payload 1)]
    (do
      (print (vector-length callables))
      (print main-func-idx)
      (print (vector-length code))
      (print entrypoint-offset)
      (print-bytes code 0 (vector-length code))
      0)))"#,
        &[],
    )
    .expect("representative native host bytes harness 実行に失敗");

    let mut lines = output.trim().lines();
    let function_start_len = lines
        .next()
        .unwrap_or_else(|| panic!("representative entrypoint offset 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("representative function-start length parse 失敗: {output}"));
    let main_func_idx = lines
        .next()
        .unwrap_or_else(|| panic!("representative main func idx 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("representative main func idx parse 失敗: {output}"));
    let declared_code_len = lines
        .next()
        .unwrap_or_else(|| panic!("representative declared code len 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("representative declared code len parse 失敗: {output}"));
    let entrypoint_offset = lines
        .next()
        .unwrap_or_else(|| panic!("representative entrypoint offset 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("representative entrypoint offset parse 失敗: {output}"));
    assert!(
        main_func_idx >= 10,
        "representative main func idx が import count 未満: main_func_idx={main_func_idx} function_starts={function_start_len}"
    );
    assert!(
        function_start_len > 0,
        "representative function-starts length が 0"
    );
    let code_bytes = lines
        .map(|line| {
            let value = line
                .parse::<i64>()
                .unwrap_or_else(|_| panic!("representative byte parse 失敗: {line}"));
            value.rem_euclid(256) as u8
        })
        .collect();
    NativeEntrypointBundle {
        function_start_len,
        main_func_idx,
        declared_code_len,
        entrypoint_offset,
        code_bytes,
    }
}

fn host_target_recursive_if_bundle_entrypoint() -> NativeEntrypointBundle {
    let output = try_compile_and_run_selfhost_fixture_entry_with_dir_and_args(
        "native-bundle-recursive-if",
        &["IR.ls", "NativeTarget.ls", "NativeCodegen.ls"],
        "Main.ls",
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [helper-load (make-instr 10 0)
        helper-if (make-instr 41 0)
        helper-reload (make-instr 10 0)
        helper-one (make-instr 1 1)
        helper-sub (make-instr 21 0)
        helper-call (make-instr 40 0)
        helper-else (make-instr 79 0)
        helper-base (make-instr 1 42)
        helper-end (make-instr 43 0)
        helper-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push (vector-new 8) helper-load)
                                helper-if)
                              helper-reload)
                            helper-one)
                          helper-sub)
                        helper-call)
                      helper-else)
                    helper-base)
        helper-ir2 (vector-push helper-ir helper-end)
        helper-meta (make-function-meta 1 0 helper-ir2)
        main-arg (make-instr 1 3)
        main-call (make-instr 40 0)
        main-ir (vector-push (vector-push (vector-new 2) main-arg) main-call)
        main-meta (make-function-meta 0 0 main-ir)
        functions (vector-push (vector-push (vector-new 2) helper-meta) main-meta)
        target (host-target)
        bundle-payload (emit-native-function-meta-bundle-entrypoint-payload-with-import-count functions 0 target)
        code (vector-get bundle-payload 0)
        entrypoint-offset (vector-get bundle-payload 1)]
    (do
      (print 2)
      (print 1)
      (print (vector-length code))
      (print entrypoint-offset)
      (print-bytes code 0 (vector-length code))
      0)))"#,
        &[],
    )
    .expect("recursive if bundle harness 実行に失敗");

    let mut lines = output.trim().lines();
    let function_start_len = lines
        .next()
        .unwrap_or_else(|| panic!("recursive if bundle function count 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("recursive if bundle function count parse 失敗: {output}"));
    let main_func_idx = lines
        .next()
        .unwrap_or_else(|| panic!("recursive if bundle main func idx 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("recursive if bundle main func idx parse 失敗: {output}"));
    let declared_code_len = lines
        .next()
        .unwrap_or_else(|| panic!("recursive if bundle declared code len 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("recursive if bundle declared code len parse 失敗: {output}"));
    let entrypoint_offset = lines
        .next()
        .unwrap_or_else(|| panic!("recursive if bundle entrypoint offset 出力が不足: {output}"))
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("recursive if bundle entrypoint offset parse 失敗: {output}"));
    let code_bytes = lines
        .map(|line: &str| {
            line.parse::<u8>()
                .unwrap_or_else(|_| panic!("recursive if bundle byte parse 失敗: {line}"))
        })
        .collect::<Vec<u8>>();

    NativeEntrypointBundle {
        function_start_len,
        main_func_idx,
        declared_code_len,
        entrypoint_offset,
        code_bytes,
    }
}

fn host_target_const_42_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr (vector-push (vector-push (vector-new 2) 1) 42)
        ir (vector-push (vector-new 1) instr)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
       0)))"#,
    )
}

fn host_target_plain_program_code_bytes(instrs: &[(u32, i64)]) -> Vec<u8> {
    let instr_bindings = instrs
        .iter()
        .enumerate()
        .map(|(idx, (opcode, operand))| format!("instr{idx} (make-instr {opcode} {operand})"))
        .collect::<Vec<_>>()
        .join("\n        ");
    let ir_expr = (0..instrs.len()).fold(format!("(vector-new {})", instrs.len()), |expr, idx| {
        format!("(vector-push {expr} instr{idx})")
    });
    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [{instr_bindings}
        ir {ir_expr}
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    );
    run_native_codegen_host_bytes_harness(&source)
}

fn host_target_local_roundtrip_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const 42)
        instr2 (make-instr 11 0)
        instr3 (make-i64-const 7)
        instr4 (make-local-get 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push (vector-new 4) instr1)
                 instr2)
               instr3)
             instr4)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_if_else_code_bytes(cond: i64) -> Vec<u8> {
    host_target_plain_program_code_bytes(&[(3, cond), (41, 0), (3, 42), (79, 0), (3, 7), (43, 0)])
}

fn host_target_if_empty_local_set_code_bytes(cond: i64) -> Vec<u8> {
    host_target_plain_program_code_bytes(&[
        (1, 7),
        (11, 0),
        (3, cond),
        (83, 0),
        (1, 42),
        (11, 0),
        (43, 0),
        (10, 0),
    ])
}

fn host_target_block_br_code_bytes() -> Vec<u8> {
    host_target_plain_program_code_bytes(&[(1, 42), (84, 0), (80, 0), (1, 7), (43, 0)])
}

fn host_target_loop_countdown_code_bytes(loop_opcode: u32) -> Vec<u8> {
    host_target_plain_program_code_bytes(&[
        (1, 5),
        (11, 0),
        (loop_opcode, 0),
        (10, 0),
        (1, 1),
        (21, 0),
        (11, 0),
        (10, 0),
        (81, 0),
        (43, 0),
        (10, 0),
    ])
}

fn host_target_i32_add_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 3 40)
        instr2 (make-instr 11 0)
        instr3 (make-instr 3 2)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 24 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_style_param1_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr (make-local-get 1)
        ir (vector-push (vector-new 1) instr)
        func (make-function-meta 2 0 ir)
        functions (vector-push (vector-new 1) func)
        normalized (normalize-selfhost-native-function-metas functions)
        target (host-target)
        code (emit-native-function-meta-bundle normalized target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_i32_mul_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 3 21)
        instr2 (make-instr 11 0)
        instr3 (make-instr 3 2)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 25 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_i32_logic_code_bytes(lhs: i32, rhs: i32, opcode: u32) -> Vec<u8> {
    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 3 {lhs})
        instr2 (make-instr 11 0)
        instr3 (make-instr 3 {rhs})
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr {opcode} 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    );
    run_native_codegen_host_bytes_harness(&source)
}

fn host_target_selfhost_root_push_drop_restore_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 3 42)
        instr2 (make-instr 11 0)
        instr3 (make-instr 3 7)
        instr4 (make-local-get 0)
        instr5 (make-instr 74 0)
        instr6 (make-instr 44 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 6) instr1)
                     instr2)
                   instr3)
                 instr4)
               instr5)
             instr6)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_root_pop_drop_restore_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 3 42)
        instr2 (make-instr 75 0)
        instr3 (make-instr 44 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr1)
               instr2)
             instr3)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_root_set_drop_restore_bottom_bundle_code_bytes() -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[
        (3, 99),
        (3, 1),
        (3, 42),
        (76, 0),
        (44, 0),
    ])
}

fn host_target_selfhost_command_line_arg_string_length_bundle_code_bytes(
    arg_index: u32,
) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {arg_index})
        instr1 (make-instr 67 0)
        instr2 (make-instr 51 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr0)
               instr1)
             instr2)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_command_line_arg_string_char_at_bundle_code_bytes(
    arg_index: u32,
    char_index: u32,
) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {arg_index})
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 {char_index})
        instr3 (make-instr 50 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push (vector-new 4) instr0)
                 instr1)
               instr2)
             instr3)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_print_bundle_code_bytes(value: u32) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {value})
        instr1 (make-instr 59 0)
        ir (vector-push
             (vector-push (vector-new 2) instr0)
             instr1)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_vector_new_length_bundle_code_bytes(capacity: u32) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {capacity})
        instr1 (make-instr 54 0)
        instr2 (make-instr 52 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr0)
               instr1)
             instr2)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_vector_push_length_bundle_code_bytes(capacity: u32, value: u32) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {capacity})
        instr1 (make-instr 54 0)
        instr2 (make-instr 3 {value})
        instr3 (make-instr 55 0)
        instr4 (make-instr 52 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push (vector-new 5) instr0)
                   instr1)
                 instr2)
               instr3)
             instr4)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_vector_push_get_bundle_code_bytes(
    capacity: u32,
    value: u32,
    index: u32,
) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {capacity})
        instr1 (make-instr 54 0)
        instr2 (make-instr 3 {value})
        instr3 (make-instr 55 0)
        instr4 (make-instr 3 {index})
        instr5 (make-instr 53 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 6) instr0)
                     instr1)
                   instr2)
                 instr3)
               instr4)
             instr5)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_vector_get_manual_seed_bundle_code_bytes(
    capacity: u32,
    length: u32,
    value: u32,
    index: u32,
) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {capacity})
        instr1 (make-instr 54 0)
        instr2 (make-instr 11 0)
        instr3 (make-instr 10 0)
        instr4 (make-instr 38 0)
        instr5 (make-instr 3 {length})
        instr6 (make-instr 46 8)
        instr7 (make-instr 10 0)
        instr8 (make-instr 38 0)
        instr9 (make-instr 3 16)
        instr10 (make-instr 24 0)
        instr11 (make-instr 3 {value})
        instr12 (make-instr 49 0)
        instr13 (make-instr 10 0)
        instr14 (make-instr 3 {index})
        instr15 (make-instr 53 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push (vector-new 16) instr0)
                                         instr1)
                                       instr2)
                                     instr3)
                                   instr4)
                                 instr5)
                               instr6)
                             instr7)
                           instr8)
                         instr9)
                       instr10)
                     instr11)
                   instr12)
                 instr13)
               instr14)
             instr15)
        func (make-function-meta 0 1 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_ref_new_get_bundle_code_bytes(value: u32) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {value})
        instr1 (make-instr 56 0)
        instr2 (make-instr 57 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr0)
               instr1)
             instr2)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_ref_set_get_bundle_code_bytes(
    initial_value: u32,
    next_value: u32,
) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 {initial_value})
        instr1 (make-instr 56 0)
        instr2 (make-instr 11 0)
        instr3 (make-instr 10 0)
        instr4 (make-instr 3 {next_value})
        instr5 (make-instr 58 0)
        instr6 (make-instr 44 0)
        instr7 (make-instr 10 0)
        instr8 (make-instr 57 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push
                           (vector-push (vector-new 9) instr0)
                           instr1)
                         instr2)
                       instr3)
                     instr4)
                   instr5)
                 instr6)
               instr7)
             instr8)
        func (make-function-meta 0 1 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_substring_length_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 1)
        instr3 (make-instr 3 4)
        instr4 (make-instr 69 0)
        instr5 (make-instr 51 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 6) instr0)
                     instr1)
                   instr2)
                 instr3)
               instr4)
             instr5)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_substring_char_at_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 1)
        instr3 (make-instr 3 4)
        instr4 (make-instr 69 0)
        instr5 (make-instr 3 0)
        instr6 (make-instr 50 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr0)
                       instr1)
                     instr2)
                   instr3)
                 instr4)
               instr5)
             instr6)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_string_concat_length_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 2)
        instr3 (make-instr 67 0)
        instr4 (make-instr 70 0)
        instr5 (make-instr 51 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 6) instr0)
                     instr1)
                   instr2)
                 instr3)
               instr4)
             instr5)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_string_concat_char_at_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 2)
        instr3 (make-instr 67 0)
        instr4 (make-instr 70 0)
        instr5 (make-instr 3 2)
        instr6 (make-instr 50 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr0)
                       instr1)
                     instr2)
                   instr3)
                 instr4)
               instr5)
             instr6)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_map_new_size_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 60 0)
        instr1 (make-instr 61 0)
        ir (vector-push
             (vector-push (vector-new 2) instr0)
             instr1)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
        0)))"#,
    )
}

fn host_target_selfhost_map_insert_size_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 60 0)
        instr1 (make-i64-const 7)
        instr2 (make-i64-const 42)
        instr3 (make-instr 62 0)
        instr4 (make-instr 61 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push (vector-new 5) instr0)
                   instr1)
                 instr2)
               instr3)
             instr4)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_map_insert_get_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 60 0)
        instr1 (make-i64-const 7)
        instr2 (make-i64-const 42)
        instr3 (make-instr 62 0)
        instr4 (make-i64-const 7)
        instr5 (make-instr 63 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 6) instr0)
                     instr1)
                   instr2)
                 instr3)
               instr4)
             instr5)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_map_insert_get_print_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 60 0)
        instr1 (make-i64-const 7)
        instr2 (make-i64-const 42)
        instr3 (make-instr 62 0)
        instr4 (make-i64-const 7)
        instr5 (make-instr 63 0)
        instr6 (make-instr 59 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr0)
                       instr1)
                     instr2)
                   instr3)
                 instr4)
               instr5)
             instr6)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_map_get_missing_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 60 0)
        instr1 (make-i64-const 99)
        instr2 (make-instr 63 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr0)
               instr1)
             instr2)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_i64_add_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const 40)
        instr2 (make-instr 11 0)
        instr3 (make-i64-const 2)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 20 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_i64_sub_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const 47)
        instr2 (make-instr 11 0)
        instr3 (make-i64-const 5)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 21 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_i64_mul_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const 21)
        instr2 (make-instr 11 0)
        instr3 (make-i64-const 2)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 22 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
       0)))"#,
    )
}

fn host_target_selfhost_file_exists_raw_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 73 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr0)
               instr1)
             instr2)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_file_exists_tagged_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 2)
        instr3 (make-instr 67 0)
        instr4 (make-instr 70 0)
        instr5 (make-instr 73 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push (vector-new 6) instr0)
                     instr1)
                   instr2)
                 instr3)
               instr4)
             instr5)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_read_file_raw_length_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 64 0)
        instr3 (make-instr 51 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push (vector-new 4) instr0)
                 instr1)
               instr2)
             instr3)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_read_file_tagged_char_at_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [instr0 (make-instr 3 1)
        instr1 (make-instr 67 0)
        instr2 (make-instr 3 2)
        instr3 (make-instr 67 0)
        instr4 (make-instr 70 0)
        instr5 (make-instr 64 0)
        instr6 (make-instr 3 0)
        instr7 (make-instr 50 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 8) instr0)
                         instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_i64_div_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const 84)
        instr2 (make-instr 11 0)
        instr3 (make-i64-const 2)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 23 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
       0)))"#,
    )
}

fn host_target_i64_rem_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const 85)
        instr2 (make-instr 11 0)
        instr3 (make-i64-const 43)
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr 28 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_single_arg_memory_program_bundle_code_bytes(instrs: &[(u32, u32)]) -> Vec<u8> {
    let instr_bindings = instrs
        .iter()
        .enumerate()
        .map(|(idx, (opcode, operand))| format!("instr{idx} (make-instr {opcode} {operand})"))
        .collect::<Vec<_>>()
        .join("\n        ");
    let ir_expr = (0..instrs.len()).fold(format!("(vector-new {})", instrs.len()), |expr, idx| {
        format!("(vector-push {expr} instr{idx})")
    });
    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn main []
  (let [{instr_bindings}
        ir {ir_expr}
        func (make-function-meta 1 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    );
    run_native_codegen_host_bytes_harness(&source)
}

fn host_target_single_arg_memory_bundle_code_bytes(opcode: u32, offset: u32) -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[(10, 0), (opcode, offset)])
}

fn host_target_single_arg_memory_store_then_load_bundle_code_bytes(
    value_opcode: u32,
    value_operand: u32,
    store_opcode: u32,
    store_offset: u32,
    load_opcode: u32,
    load_offset: u32,
) -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[
        (10, 0),
        (value_opcode, value_operand),
        (store_opcode, store_offset),
        (10, 0),
        (load_opcode, load_offset),
    ])
}

fn host_target_single_arg_i64_store_then_load_double_drop_bundle_code_bytes() -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[
        (3, 5),
        (3, 7),
        (10, 0),
        (3, 42),
        (49, 8),
        (10, 0),
        (48, 8),
        (44, 0),
        (44, 0),
    ])
}

fn host_target_single_arg_memory_fill_load_sum_bundle_code_bytes(
    fill_value: u32,
    fill_len: u32,
) -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[
        (10, 0),
        (3, fill_value),
        (3, fill_len),
        (78, 0),
        (10, 0),
        (47, 4),
        (10, 0),
        (47, 5),
        (24, 0),
    ])
}

fn host_target_single_arg_memory_copy_load_sum_bundle_code_bytes(copy_len: u32) -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[
        (10, 0),
        (1, 8),
        (20, 0),
        (10, 0),
        (3, copy_len),
        (77, 0),
        (10, 0),
        (47, 12),
        (10, 0),
        (47, 13),
        (24, 0),
    ])
}

fn host_target_single_arg_memory_fill_drop_restored_bottom_bundle_code_bytes() -> Vec<u8> {
    host_target_single_arg_memory_program_bundle_code_bytes(&[
        (3, 5),
        (3, 7),
        (10, 0),
        (3, 42),
        (3, 1),
        (78, 0),
        (44, 0),
    ])
}

fn host_target_i64_compare_code_bytes(lhs: i64, rhs: i64, opcode: u32) -> Vec<u8> {
    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-i64-const {lhs})
        instr2 (make-instr 11 0)
        instr3 (make-i64-const {rhs})
        instr4 (make-instr 11 1)
        instr5 (make-local-get 0)
        instr6 (make-local-get 1)
        instr7 (make-instr {opcode} 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 7) instr1)
                       instr2)
                     instr3)
                   instr4)
                 instr5)
               instr6)
             instr7)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        lhs = lhs,
        rhs = rhs,
        opcode = opcode
    );

    run_native_codegen_host_bytes_harness(&source)
}

fn host_target_drop_restore_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [instr1 (make-instr 3 42)
        instr2 (make-instr 11 0)
        instr3 (make-instr 3 7)
        instr4 (make-local-get 0)
        instr5 (make-instr 44 0)
        ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push (vector-new 5) instr1)
                   instr2)
                 instr3)
               instr4)
             instr5)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push (vector-new 1) (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-instr 3 42))
        functions (vector-push (vector-push (vector-new 2) caller-ir) callee-ir)
        target (host-target)
        code (emit-native-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push (vector-new 2) (make-i64-const 42))
                    (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 1 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_import_prefixed_direct_call_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [import-meta (make-function-meta 0 0 (vector-new 0))
        caller-ir (vector-push
                    (vector-push (vector-new 2) (make-i64-const 42))
                    (make-call 2))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 1 0 callee-ir)
        functions (vector-push
                    (vector-push
                      (vector-push (vector-new 3) import-meta)
                      caller)
                    callee)
        target (host-target)
        code (emit-native-function-meta-bundle-with-import-count functions 1 target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_import_call_stub_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [import-meta (make-function-meta 0 0 (vector-new 0))
        caller-ir (vector-push
                    (vector-push (vector-new 2) (make-i64-const 42))
                    (make-call 0))
        caller (make-function-meta 0 0 caller-ir)
        functions (vector-push
                    (vector-push (vector-new 2) import-meta)
                    caller)
        target (host-target)
        code (emit-native-function-meta-bundle-with-import-count functions 1 target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_selfhost_alloc_import_bundle_code_bytes(size: u32) -> Vec<u8> {
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [print-import (make-function-meta 1 0 (vector-new 0))
        alloc-import (make-function-meta 1 0 (vector-new 0))
        instr0 (make-i64-const {size})
        instr1 (make-call 1)
        instr2 (make-instr 59 0)
        ir (vector-push
             (vector-push
               (vector-push (vector-new 3) instr0)
               instr1)
             instr2)
        caller (make-function-meta 0 0 ir)
        functions (vector-push
                    (vector-push
                      (vector-push (vector-new 3) print-import)
                      alloc-import)
                    caller)
        target (host-target)
        code (emit-native-function-meta-bundle-with-import-count functions 2 target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_alloc_runtime_bundle_code_bytes(
    local_count: u32,
    instr_exprs: &[String],
) -> Vec<u8> {
    let mut instr_bindings = String::new();
    let mut ir_bindings = String::new();
    for (idx, expr) in instr_exprs.iter().enumerate() {
        instr_bindings.push_str(&format!("        instr{idx} {expr}\n"));
        if idx == 0 {
            ir_bindings.push_str(&format!(
                "        ir0 (vector-push (vector-new {}) instr0)\n",
                instr_exprs.len()
            ));
        } else {
            ir_bindings.push_str(&format!(
                "        ir{idx} (vector-push ir{} instr{idx})\n",
                idx - 1
            ));
        }
    }
    let ir_name = if instr_exprs.is_empty() {
        "(vector-new 0)".to_string()
    } else {
        format!("ir{}", instr_exprs.len() - 1)
    };

    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [print-import (make-function-meta 1 0 (vector-new 0))
        alloc-import (make-function-meta 1 0 (vector-new 0))
{instr_bindings}{ir_bindings}        caller (make-function-meta 0 {local_count} {ir_name})
        functions (vector-push
                    (vector-push
                      (vector-push (vector-new 3) print-import)
                      alloc-import)
                    caller)
        target (host-target)
        code (emit-native-function-meta-bundle-with-import-count functions 2 target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    ))
}

fn host_target_selfhost_alloc_i32_store_load_bundle_code_bytes(value: u32) -> Vec<u8> {
    host_target_selfhost_alloc_runtime_bundle_code_bytes(
        1,
        &[
            "(make-i64-const 4)".to_string(),
            "(make-call 1)".to_string(),
            "(make-instr 11 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            format!("(make-i32-const {value})"),
            "(make-i32-store 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-instr 45 0)".to_string(),
            "(make-instr 59 0)".to_string(),
        ],
    )
}

fn host_target_selfhost_alloc_i32_load8_u_bundle_code_bytes() -> Vec<u8> {
    host_target_selfhost_alloc_runtime_bundle_code_bytes(
        1,
        &[
            "(make-i64-const 4)".to_string(),
            "(make-call 1)".to_string(),
            "(make-instr 11 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-i32-const 25185)".to_string(),
            "(make-i32-store 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-instr 47 1)".to_string(),
            "(make-instr 59 0)".to_string(),
        ],
    )
}

fn host_target_selfhost_alloc_i64_store_load_bundle_code_bytes(value: u32) -> Vec<u8> {
    host_target_selfhost_alloc_runtime_bundle_code_bytes(
        1,
        &[
            "(make-i64-const 8)".to_string(),
            "(make-call 1)".to_string(),
            "(make-instr 11 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            format!("(make-i64-const {value})"),
            "(make-instr 49 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-instr 48 0)".to_string(),
            "(make-instr 59 0)".to_string(),
        ],
    )
}

fn host_target_selfhost_alloc_memory_copy_bundle_code_bytes() -> Vec<u8> {
    host_target_selfhost_alloc_runtime_bundle_code_bytes(
        2,
        &[
            "(make-i64-const 4)".to_string(),
            "(make-call 1)".to_string(),
            "(make-instr 11 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-i32-const 25185)".to_string(),
            "(make-i32-store 0)".to_string(),
            "(make-i64-const 4)".to_string(),
            "(make-call 1)".to_string(),
            "(make-instr 11 1)".to_string(),
            "(make-instr 10 1)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-i32-const 2)".to_string(),
            "(make-instr 77 0)".to_string(),
            "(make-instr 10 1)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-instr 47 1)".to_string(),
            "(make-instr 59 0)".to_string(),
        ],
    )
}

fn host_target_selfhost_alloc_memory_fill_bundle_code_bytes(fill_value: u32) -> Vec<u8> {
    host_target_selfhost_alloc_runtime_bundle_code_bytes(
        1,
        &[
            "(make-i64-const 4)".to_string(),
            "(make-call 1)".to_string(),
            "(make-instr 11 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            format!("(make-i32-const {fill_value})"),
            "(make-i32-const 3)".to_string(),
            "(make-instr 78 0)".to_string(),
            "(make-instr 10 0)".to_string(),
            "(make-instr 38 0)".to_string(),
            "(make-instr 47 2)".to_string(),
            "(make-instr 59 0)".to_string(),
        ],
    )
}

fn host_target_direct_call_two_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push (vector-new 3) (make-instr 3 40))
                      (make-instr 3 2))
                    (make-call 1))
        callee-ir (vector-push
                    (vector-push
                      (vector-push (vector-new 3) (make-local-get 0))
                      (make-local-get 1))
                            (make-instr 24 0)))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 2 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_three_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push (vector-new 4) (make-instr 3 40))
                        (make-instr 3 2))
                      (make-instr 3 5))
                    (make-call 1))
        callee-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push (vector-new 5) (make-local-get 0))
                          (make-local-get 1))
                        (make-instr 24 0))
                      (make-local-get 2))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 3 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_four_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push (vector-new 5) (make-instr 3 40))
                          (make-instr 3 2))
                        (make-instr 3 5))
                      (make-instr 3 7))
                    (make-call 1))
        callee-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push (vector-new 7) (make-local-get 0))
                              (make-local-get 1))
                            (make-instr 24 0)))
                          (make-local-get 2))
                        (make-instr 24 0))
                      (make-local-get 3))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 4 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_five_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push (vector-new 6) (make-instr 3 40))
                            (make-instr 3 2))
                          (make-instr 3 5))
                        (make-instr 3 7))
                      (make-instr 3 11))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push (vector-new 9) (make-local-get 0))
                                     (make-local-get 1))
                                   (make-instr 24 0))
                                 (make-local-get 2))
                               (make-instr 24 0))
                             (make-local-get 3))
                           (make-instr 24 0))
                         (make-local-get 4))
        callee-ir (vector-push callee-ir-base (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 5 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_six_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push (vector-new 7) (make-instr 3 40))
                              (make-instr 3 2))
                            (make-instr 3 5))
                          (make-instr 3 7))
                        (make-instr 3 11))
                      (make-instr 3 14))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push (vector-new 11) (make-local-get 0))
                                       (make-local-get 1))
                                     (make-instr 24 0))
                                   (make-local-get 2))
                                 (make-instr 24 0))
                               (make-local-get 3))
                             (make-instr 24 0))
                           (make-local-get 4))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-base (make-local-get 5))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 6 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_seven_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push (vector-new 8) (make-instr 3 40))
                                (make-instr 3 2))
                              (make-instr 3 5))
                            (make-instr 3 7))
                          (make-instr 3 11))
                        (make-instr 3 14))
                      (make-instr 3 17))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push (vector-new 13) (make-local-get 0))
                                      (make-local-get 1))
                                    (make-instr 24 0))
                                     (make-local-get 2))
                                   (make-instr 24 0))
                                 (make-local-get 3))
                               (make-instr 24 0))
                             (make-local-get 4))
                           (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-base (make-local-get 5))
                        (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-mid (make-local-get 6))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 7 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_eight_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push (vector-new 9) (make-instr 3 40))
                                  (make-instr 3 2))
                                (make-instr 3 5))
                              (make-instr 3 7))
                            (make-instr 3 11))
                          (make-instr 3 14))
                        (make-instr 3 17))
                      (make-instr 3 19))
                    (make-call 1))
        callee-ir-base (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push (vector-new 15) (make-local-get 0))
                                           (make-local-get 1))
                                         (make-instr 24 0))
                                       (make-local-get 2))
                                     (make-instr 24 0))
                                   (make-local-get 3))
                                 (make-instr 24 0))
                               (make-local-get 4))
                             (make-instr 24 0))
                           (make-local-get 5))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-base (make-local-get 6))
                        (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-mid (make-local-get 7))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 8 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_nine_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push (vector-new 10) (make-instr 3 40))
                                    (make-instr 3 2))
                                  (make-instr 3 5))
                                (make-instr 3 7))
                              (make-instr 3 11))
                            (make-instr 3 14))
                          (make-instr 3 17))
                        (make-instr 3 19))
                      (make-instr 3 23))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push (vector-new 17) (make-local-get 0))
                                             (make-local-get 1))
                                           (make-instr 24 0))
                                         (make-local-get 2))
                                       (make-instr 24 0))
                                     (make-local-get 3))
                                   (make-instr 24 0))
                                 (make-local-get 4))
                               (make-instr 24 0))
                             (make-local-get 5))
                           (make-instr 24 0))
                         (make-local-get 6))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-instr 24 0))
                        (make-local-get 7))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-instr 24 0))
                         (make-local-get 8))
        callee-ir (vector-push callee-ir-tail (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 9 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_ten_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push (vector-new 11) (make-instr 3 40))
                                      (make-instr 3 2))
                                    (make-instr 3 5))
                                  (make-instr 3 7))
                                (make-instr 3 11))
                              (make-instr 3 14))
                            (make-instr 3 17))
                          (make-instr 3 19))
                        (make-instr 3 23))
                      (make-instr 3 29))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push (vector-new 17) (make-local-get 0))
                                           (make-local-get 1))
                                         (make-instr 24 0))
                                       (make-local-get 2))
                                     (make-instr 24 0))
                                   (make-local-get 3))
                                 (make-instr 24 0))
                               (make-local-get 4))
                             (make-instr 24 0))
                           (make-local-get 5))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 6))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 7))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 8))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-more (make-local-get 9))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 10 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_eleven_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push (vector-new 12) (make-instr 3 40))
                                        (make-instr 3 2))
                                      (make-instr 3 5))
                                    (make-instr 3 7))
                                  (make-instr 3 11))
                                (make-instr 3 14))
                              (make-instr 3 17))
                            (make-instr 3 19))
                          (make-instr 3 23))
                        (make-instr 3 29))
                      (make-instr 3 31))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 21) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-more (make-local-get 10))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 11 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twelve_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push (vector-new 13) (make-instr 3 40))
                                          (make-instr 3 2))
                                        (make-instr 3 5))
                                      (make-instr 3 7))
                                    (make-instr 3 11))
                                  (make-instr 3 14))
                                (make-instr 3 17))
                              (make-instr 3 19))
                            (make-instr 3 23))
                          (make-instr 3 29))
                        (make-instr 3 31))
                      (make-instr 3 37))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 23) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-last (make-local-get 11))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 12 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirteen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push (vector-new 14) (make-instr 3 40))
                                            (make-instr 3 2))
                                          (make-instr 3 5))
                                        (make-instr 3 7))
                                      (make-instr 3 11))
                                    (make-instr 3 13))
                                  (make-instr 3 14))
                                (make-instr 3 17))
                              (make-instr 3 19))
                            (make-instr 3 23))
                          (make-instr 3 29))
                        (make-instr 3 31))
                      (make-instr 3 37))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 25) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next (make-local-get 12))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 13 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_fourteen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push (vector-new 15) (make-instr 3 31))
                                              (make-instr 3 2))
                                            (make-instr 3 3))
                                          (make-instr 3 5))
                                        (make-instr 3 7))
                                      (make-instr 3 11))
                                    (make-instr 3 13))
                                  (make-instr 3 14))
                                (make-instr 3 17))
                              (make-instr 3 19))
                            (make-instr 3 23))
                          (make-instr 3 29))
                        (make-instr 3 31))
                      (make-instr 3 37))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 27) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next2 (make-local-get 13))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 14 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_fifteen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push (vector-new 16) (make-instr 3 31))
                                                (make-instr 3 2))
                                              (make-instr 3 3))
                                            (make-instr 3 5))
                                          (make-instr 3 7))
                                        (make-instr 3 11))
                                      (make-instr 3 13))
                                    (make-instr 3 14))
                                  (make-instr 3 17))
                                (make-instr 3 19))
                              (make-instr 3 23))
                            (make-instr 3 29))
                          (make-instr 3 31))
                        (make-instr 3 37))
                      (make-instr 3 1))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 29) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next3 (make-local-get 14))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 15 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_sixteen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push
                                                  (vector-push (vector-new 17) (make-instr 3 31))
                                                  (make-instr 3 2))
                                                (make-instr 3 3))
                                              (make-instr 3 5))
                                            (make-instr 3 7))
                                          (make-instr 3 11))
                                        (make-instr 3 13))
                                      (make-instr 3 14))
                                    (make-instr 3 17))
                                  (make-instr 3 19))
                                (make-instr 3 23))
                              (make-instr 3 29))
                            (make-instr 3 31))
                          (make-instr 3 37))
                        (make-instr 3 1))
                      (make-instr 3 2))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 31) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir-next4 (vector-push
                          (vector-push callee-ir-next3 (make-local-get 14))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next4 (make-local-get 15))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 16 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_seventeen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push
                                                  (vector-push
                                                    (vector-push (vector-new 18) (make-instr 3 31))
                                                    (make-instr 3 2))
                                                  (make-instr 3 3))
                                                (make-instr 3 5))
                                              (make-instr 3 7))
                                            (make-instr 3 11))
                                          (make-instr 3 13))
                                        (make-instr 3 14))
                                      (make-instr 3 17))
                                    (make-instr 3 19))
                                  (make-instr 3 23))
                                (make-instr 3 29))
                              (make-instr 3 31))
                            (make-instr 3 37))
                          (make-instr 3 1))
                        (make-instr 3 2))
                      (make-instr 3 4))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 33) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir-next4 (vector-push
                          (vector-push callee-ir-next3 (make-local-get 14))
                          (make-instr 24 0))
        callee-ir-next5 (vector-push
                          (vector-push callee-ir-next4 (make-local-get 15))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next5 (make-local-get 16))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 17 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_eighteen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push
                                                  (vector-push
                                                    (vector-push
                                                      (vector-push (vector-new 19) (make-instr 3 31))
                                                      (make-instr 3 2))
                                                    (make-instr 3 3))
                                                  (make-instr 3 5))
                                                (make-instr 3 7))
                                              (make-instr 3 11))
                                            (make-instr 3 13))
                                          (make-instr 3 14))
                                        (make-instr 3 17))
                                      (make-instr 3 19))
                                    (make-instr 3 23))
                                  (make-instr 3 29))
                                (make-instr 3 31))
                              (make-instr 3 37))
                            (make-instr 3 1))
                          (make-instr 3 2))
                        (make-instr 3 4))
                      (make-instr 3 3))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 35) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir-next4 (vector-push
                          (vector-push callee-ir-next3 (make-local-get 14))
                          (make-instr 24 0))
        callee-ir-next5 (vector-push
                          (vector-push callee-ir-next4 (make-local-get 15))
                          (make-instr 24 0))
        callee-ir-next6 (vector-push
                          (vector-push callee-ir-next5 (make-local-get 16))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next6 (make-local-get 17))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 18 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_nineteen_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push
                            (vector-push
                              (vector-push
                                (vector-push
                                  (vector-push
                                    (vector-push
                                      (vector-push
                                        (vector-push
                                          (vector-push
                                            (vector-push
                                              (vector-push
                                                (vector-push
                                                  (vector-push
                                                    (vector-push
                                                      (vector-push
                                                        (vector-push (vector-new 20) (make-instr 3 31))
                                                        (make-instr 3 2))
                                                      (make-instr 3 3))
                                                    (make-instr 3 5))
                                                  (make-instr 3 7))
                                                (make-instr 3 11))
                                              (make-instr 3 13))
                                            (make-instr 3 14))
                                          (make-instr 3 17))
                                        (make-instr 3 19))
                                      (make-instr 3 23))
                                    (make-instr 3 29))
                                  (make-instr 3 31))
                                (make-instr 3 37))
                              (make-instr 3 1))
                            (make-instr 3 2))
                          (make-instr 3 4))
                        (make-instr 3 3))
                      (make-instr 3 1))
                    (make-call 1))
        callee-ir-head (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push
                                         (vector-push
                                           (vector-push
                                             (vector-push
                                               (vector-push (vector-new 37) (make-local-get 0))
                                               (make-local-get 1))
                                             (make-instr 24 0))
                                           (make-local-get 2))
                                         (make-instr 24 0))
                                       (make-local-get 3))
                                     (make-instr 24 0))
                                   (make-local-get 4))
                                 (make-instr 24 0))
                               (make-local-get 5))
                             (make-instr 24 0))
                           (make-local-get 6))
                         (make-instr 24 0))
        callee-ir-mid (vector-push
                        (vector-push callee-ir-head (make-local-get 7))
                        (make-instr 24 0))
        callee-ir-tail (vector-push
                         (vector-push callee-ir-mid (make-local-get 8))
                         (make-instr 24 0))
        callee-ir-more (vector-push
                         (vector-push callee-ir-tail (make-local-get 9))
                         (make-instr 24 0))
        callee-ir-last (vector-push
                         (vector-push callee-ir-more (make-local-get 10))
                         (make-instr 24 0))
        callee-ir-next (vector-push
                         (vector-push callee-ir-last (make-local-get 11))
                         (make-instr 24 0))
        callee-ir-next2 (vector-push
                          (vector-push callee-ir-next (make-local-get 12))
                          (make-instr 24 0))
        callee-ir-next3 (vector-push
                          (vector-push callee-ir-next2 (make-local-get 13))
                          (make-instr 24 0))
        callee-ir-next4 (vector-push
                          (vector-push callee-ir-next3 (make-local-get 14))
                          (make-instr 24 0))
        callee-ir-next5 (vector-push
                          (vector-push callee-ir-next4 (make-local-get 15))
                          (make-instr 24 0))
        callee-ir-next6 (vector-push
                          (vector-push callee-ir-next5 (make-local-get 16))
                          (make-instr 24 0))
        callee-ir-next7 (vector-push
                          (vector-push callee-ir-next6 (make-local-get 17))
                          (make-instr 24 0))
        callee-ir (vector-push
                    (vector-push callee-ir-next7 (make-local-get 18))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 19 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 21) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir (vector-push caller-ir19 (make-call 1))
        callee-ir0 (vector-push (vector-new 39) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir (vector-push callee-ir37 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 20 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_one_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 22) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir (vector-push caller-ir20 (make-call 1))
        callee-ir0 (vector-push (vector-new 41) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir (vector-push callee-ir39 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 21 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_two_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 23) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir (vector-push caller-ir21 (make-call 1))
        callee-ir0 (vector-push (vector-new 43) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir (vector-push callee-ir41 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 22 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
       0)))"#,
    )
}

fn host_target_direct_call_twenty_three_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 24) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir (vector-push caller-ir22 (make-call 1))
        callee-ir0 (vector-push (vector-new 45) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir (vector-push callee-ir43 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 23 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_four_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 25) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir (vector-push caller-ir23 (make-call 1))
        callee-ir0 (vector-push (vector-new 47) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir (vector-push callee-ir45 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 24 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_five_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 26) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir (vector-push caller-ir24 (make-call 1))
        callee-ir0 (vector-push (vector-new 49) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir (vector-push callee-ir47 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 25 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_six_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 27) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir (vector-push caller-ir25 (make-call 1))
        callee-ir0 (vector-push (vector-new 51) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir (vector-push callee-ir49 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 26 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_seven_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 28) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir (vector-push caller-ir26 (make-call 1))
        callee-ir0 (vector-push (vector-new 53) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir (vector-push callee-ir51 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 27 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_eight_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 29) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir (vector-push caller-ir27 (make-call 1))
        callee-ir0 (vector-push (vector-new 55) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir (vector-push callee-ir53 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 28 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_twenty_nine_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 30) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir (vector-push caller-ir28 (make-call 1))
        callee-ir0 (vector-push (vector-new 57) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir (vector-push callee-ir55 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 29 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 31) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir (vector-push caller-ir29 (make-call 1))
        callee-ir0 (vector-push (vector-new 59) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir (vector-push callee-ir57 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 30 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_one_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 32) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir (vector-push caller-ir30 (make-call 1))
        callee-ir0 (vector-push (vector-new 61) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir (vector-push callee-ir59 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 31 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_two_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 33) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir (vector-push caller-ir31 (make-call 1))
        callee-ir0 (vector-push (vector-new 63) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir (vector-push callee-ir61 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 32 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_three_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 34) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir (vector-push caller-ir32 (make-call 1))
        callee-ir0 (vector-push (vector-new 65) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir (vector-push callee-ir63 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 33 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_four_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 35) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir (vector-push caller-ir33 (make-call 1))
        callee-ir0 (vector-push (vector-new 67) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir (vector-push callee-ir65 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 34 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_five_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 36) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir (vector-push caller-ir34 (make-call 1))
        callee-ir0 (vector-push (vector-new 69) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir (vector-push callee-ir67 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 35 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_six_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 37) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir (vector-push caller-ir35 (make-call 1))
        callee-ir0 (vector-push (vector-new 71) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir (vector-push callee-ir69 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 36 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_seven_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 38) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir (vector-push caller-ir36 (make-call 1))
        callee-ir0 (vector-push (vector-new 73) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir (vector-push callee-ir71 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 37 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_eight_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 39) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir (vector-push caller-ir37 (make-call 1))
        callee-ir0 (vector-push (vector-new 75) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir (vector-push callee-ir73 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 38 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_thirty_nine_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 40) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir (vector-push caller-ir38 (make-call 1))
        callee-ir0 (vector-push (vector-new 77) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir (vector-push callee-ir75 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 39 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 41) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir (vector-push caller-ir39 (make-call 1))
        callee-ir0 (vector-push (vector-new 79) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir (vector-push callee-ir77 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 40 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_one_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 42) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir (vector-push caller-ir40 (make-call 1))
        callee-ir0 (vector-push (vector-new 81) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir (vector-push callee-ir79 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 41 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_two_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 43) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir (vector-push caller-ir41 (make-call 1))
        callee-ir0 (vector-push (vector-new 83) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir (vector-push callee-ir81 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 42 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_three_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 44) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir42 (vector-push caller-ir41 (make-instr 3 21))
        caller-ir (vector-push caller-ir42 (make-call 1))
        callee-ir0 (vector-push (vector-new 85) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir82 (vector-push callee-ir81 (make-instr 24 0))
        callee-ir83 (vector-push callee-ir82 (make-local-get 42))
        callee-ir (vector-push callee-ir83 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 43 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_four_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 45) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir42 (vector-push caller-ir41 (make-instr 3 21))
        caller-ir43 (vector-push caller-ir42 (make-instr 3 22))
        caller-ir (vector-push caller-ir43 (make-call 1))
        callee-ir0 (vector-push (vector-new 87) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir82 (vector-push callee-ir81 (make-instr 24 0))
        callee-ir83 (vector-push callee-ir82 (make-local-get 42))
        callee-ir84 (vector-push callee-ir83 (make-instr 24 0))
        callee-ir85 (vector-push callee-ir84 (make-local-get 43))
        callee-ir (vector-push callee-ir85 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 44 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_five_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 46) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir42 (vector-push caller-ir41 (make-instr 3 21))
        caller-ir43 (vector-push caller-ir42 (make-instr 3 22))
        caller-ir44 (vector-push caller-ir43 (make-instr 3 23))
        caller-ir (vector-push caller-ir44 (make-call 1))
        callee-ir0 (vector-push (vector-new 89) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir82 (vector-push callee-ir81 (make-instr 24 0))
        callee-ir83 (vector-push callee-ir82 (make-local-get 42))
        callee-ir84 (vector-push callee-ir83 (make-instr 24 0))
        callee-ir85 (vector-push callee-ir84 (make-local-get 43))
        callee-ir86 (vector-push callee-ir85 (make-instr 24 0))
        callee-ir87 (vector-push callee-ir86 (make-local-get 44))
        callee-ir (vector-push callee-ir87 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 45 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_six_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 47) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir42 (vector-push caller-ir41 (make-instr 3 21))
        caller-ir43 (vector-push caller-ir42 (make-instr 3 22))
        caller-ir44 (vector-push caller-ir43 (make-instr 3 23))
        caller-ir45 (vector-push caller-ir44 (make-instr 3 24))
        caller-ir (vector-push caller-ir45 (make-call 1))
        callee-ir0 (vector-push (vector-new 91) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir82 (vector-push callee-ir81 (make-instr 24 0))
        callee-ir83 (vector-push callee-ir82 (make-local-get 42))
        callee-ir84 (vector-push callee-ir83 (make-instr 24 0))
        callee-ir85 (vector-push callee-ir84 (make-local-get 43))
        callee-ir86 (vector-push callee-ir85 (make-instr 24 0))
        callee-ir87 (vector-push callee-ir86 (make-local-get 44))
        callee-ir88 (vector-push callee-ir87 (make-instr 24 0))
        callee-ir89 (vector-push callee-ir88 (make-local-get 45))
        callee-ir (vector-push callee-ir89 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 46 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_seven_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 48) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir42 (vector-push caller-ir41 (make-instr 3 21))
        caller-ir43 (vector-push caller-ir42 (make-instr 3 22))
        caller-ir44 (vector-push caller-ir43 (make-instr 3 23))
        caller-ir45 (vector-push caller-ir44 (make-instr 3 24))
        caller-ir46 (vector-push caller-ir45 (make-instr 3 25))
        caller-ir (vector-push caller-ir46 (make-call 1))
        callee-ir0 (vector-push (vector-new 93) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir82 (vector-push callee-ir81 (make-instr 24 0))
        callee-ir83 (vector-push callee-ir82 (make-local-get 42))
        callee-ir84 (vector-push callee-ir83 (make-instr 24 0))
        callee-ir85 (vector-push callee-ir84 (make-local-get 43))
        callee-ir86 (vector-push callee-ir85 (make-instr 24 0))
        callee-ir87 (vector-push callee-ir86 (make-local-get 44))
        callee-ir88 (vector-push callee-ir87 (make-instr 24 0))
        callee-ir89 (vector-push callee-ir88 (make-local-get 45))
        callee-ir90 (vector-push callee-ir89 (make-instr 24 0))
        callee-ir91 (vector-push callee-ir90 (make-local-get 46))
        callee-ir (vector-push callee-ir91 (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 47 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_forty_eight_arg_bundle_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir0 (vector-push (vector-new 49) (make-instr 3 31))
        caller-ir1 (vector-push caller-ir0 (make-instr 3 2))
        caller-ir2 (vector-push caller-ir1 (make-instr 3 3))
        caller-ir3 (vector-push caller-ir2 (make-instr 3 5))
        caller-ir4 (vector-push caller-ir3 (make-instr 3 7))
        caller-ir5 (vector-push caller-ir4 (make-instr 3 11))
        caller-ir6 (vector-push caller-ir5 (make-instr 3 13))
        caller-ir7 (vector-push caller-ir6 (make-instr 3 14))
        caller-ir8 (vector-push caller-ir7 (make-instr 3 17))
        caller-ir9 (vector-push caller-ir8 (make-instr 3 19))
        caller-ir10 (vector-push caller-ir9 (make-instr 3 23))
        caller-ir11 (vector-push caller-ir10 (make-instr 3 29))
        caller-ir12 (vector-push caller-ir11 (make-instr 3 31))
        caller-ir13 (vector-push caller-ir12 (make-instr 3 37))
        caller-ir14 (vector-push caller-ir13 (make-instr 3 1))
        caller-ir15 (vector-push caller-ir14 (make-instr 3 2))
        caller-ir16 (vector-push caller-ir15 (make-instr 3 4))
        caller-ir17 (vector-push caller-ir16 (make-instr 3 3))
        caller-ir18 (vector-push caller-ir17 (make-instr 3 1))
        caller-ir19 (vector-push caller-ir18 (make-instr 3 1))
        caller-ir20 (vector-push caller-ir19 (make-instr 3 1))
        caller-ir21 (vector-push caller-ir20 (make-instr 3 2))
        caller-ir22 (vector-push caller-ir21 (make-instr 3 41))
        caller-ir23 (vector-push caller-ir22 (make-instr 3 8))
        caller-ir24 (vector-push caller-ir23 (make-instr 3 13))
        caller-ir25 (vector-push caller-ir24 (make-instr 3 5))
        caller-ir26 (vector-push caller-ir25 (make-instr 3 7))
        caller-ir27 (vector-push caller-ir26 (make-instr 3 11))
        caller-ir28 (vector-push caller-ir27 (make-instr 3 3))
        caller-ir29 (vector-push caller-ir28 (make-instr 3 2))
        caller-ir30 (vector-push caller-ir29 (make-instr 3 4))
        caller-ir31 (vector-push caller-ir30 (make-instr 3 6))
        caller-ir32 (vector-push caller-ir31 (make-instr 3 10))
        caller-ir33 (vector-push caller-ir32 (make-instr 3 12))
        caller-ir34 (vector-push caller-ir33 (make-instr 3 13))
        caller-ir35 (vector-push caller-ir34 (make-instr 3 14))
        caller-ir36 (vector-push caller-ir35 (make-instr 3 15))
        caller-ir37 (vector-push caller-ir36 (make-instr 3 16))
        caller-ir38 (vector-push caller-ir37 (make-instr 3 17))
        caller-ir39 (vector-push caller-ir38 (make-instr 3 18))
        caller-ir40 (vector-push caller-ir39 (make-instr 3 19))
        caller-ir41 (vector-push caller-ir40 (make-instr 3 20))
        caller-ir42 (vector-push caller-ir41 (make-instr 3 21))
        caller-ir43 (vector-push caller-ir42 (make-instr 3 22))
        caller-ir44 (vector-push caller-ir43 (make-instr 3 23))
        caller-ir45 (vector-push caller-ir44 (make-instr 3 24))
        caller-ir46 (vector-push caller-ir45 (make-instr 3 25))
        caller-ir47 (vector-push caller-ir46 (make-instr 3 26))
        caller-ir (vector-push caller-ir47 (make-call 1))
        callee-ir0 (vector-push (vector-new 95) (make-local-get 0))
        callee-ir1 (vector-push callee-ir0 (make-local-get 1))
        callee-ir2 (vector-push callee-ir1 (make-instr 24 0))
        callee-ir3 (vector-push callee-ir2 (make-local-get 2))
        callee-ir4 (vector-push callee-ir3 (make-instr 24 0))
        callee-ir5 (vector-push callee-ir4 (make-local-get 3))
        callee-ir6 (vector-push callee-ir5 (make-instr 24 0))
        callee-ir7 (vector-push callee-ir6 (make-local-get 4))
        callee-ir8 (vector-push callee-ir7 (make-instr 24 0))
        callee-ir9 (vector-push callee-ir8 (make-local-get 5))
        callee-ir10 (vector-push callee-ir9 (make-instr 24 0))
        callee-ir11 (vector-push callee-ir10 (make-local-get 6))
        callee-ir12 (vector-push callee-ir11 (make-instr 24 0))
        callee-ir13 (vector-push callee-ir12 (make-local-get 7))
        callee-ir14 (vector-push callee-ir13 (make-instr 24 0))
        callee-ir15 (vector-push callee-ir14 (make-local-get 8))
        callee-ir16 (vector-push callee-ir15 (make-instr 24 0))
        callee-ir17 (vector-push callee-ir16 (make-local-get 9))
        callee-ir18 (vector-push callee-ir17 (make-instr 24 0))
        callee-ir19 (vector-push callee-ir18 (make-local-get 10))
        callee-ir20 (vector-push callee-ir19 (make-instr 24 0))
        callee-ir21 (vector-push callee-ir20 (make-local-get 11))
        callee-ir22 (vector-push callee-ir21 (make-instr 24 0))
        callee-ir23 (vector-push callee-ir22 (make-local-get 12))
        callee-ir24 (vector-push callee-ir23 (make-instr 24 0))
        callee-ir25 (vector-push callee-ir24 (make-local-get 13))
        callee-ir26 (vector-push callee-ir25 (make-instr 24 0))
        callee-ir27 (vector-push callee-ir26 (make-local-get 14))
        callee-ir28 (vector-push callee-ir27 (make-instr 24 0))
        callee-ir29 (vector-push callee-ir28 (make-local-get 15))
        callee-ir30 (vector-push callee-ir29 (make-instr 24 0))
        callee-ir31 (vector-push callee-ir30 (make-local-get 16))
        callee-ir32 (vector-push callee-ir31 (make-instr 24 0))
        callee-ir33 (vector-push callee-ir32 (make-local-get 17))
        callee-ir34 (vector-push callee-ir33 (make-instr 24 0))
        callee-ir35 (vector-push callee-ir34 (make-local-get 18))
        callee-ir36 (vector-push callee-ir35 (make-instr 24 0))
        callee-ir37 (vector-push callee-ir36 (make-local-get 19))
        callee-ir38 (vector-push callee-ir37 (make-instr 24 0))
        callee-ir39 (vector-push callee-ir38 (make-local-get 20))
        callee-ir40 (vector-push callee-ir39 (make-instr 24 0))
        callee-ir41 (vector-push callee-ir40 (make-local-get 21))
        callee-ir42 (vector-push callee-ir41 (make-instr 24 0))
        callee-ir43 (vector-push callee-ir42 (make-local-get 22))
        callee-ir44 (vector-push callee-ir43 (make-instr 24 0))
        callee-ir45 (vector-push callee-ir44 (make-local-get 23))
        callee-ir46 (vector-push callee-ir45 (make-instr 24 0))
        callee-ir47 (vector-push callee-ir46 (make-local-get 24))
        callee-ir48 (vector-push callee-ir47 (make-instr 24 0))
        callee-ir49 (vector-push callee-ir48 (make-local-get 25))
        callee-ir50 (vector-push callee-ir49 (make-instr 24 0))
        callee-ir51 (vector-push callee-ir50 (make-local-get 26))
        callee-ir52 (vector-push callee-ir51 (make-instr 24 0))
        callee-ir53 (vector-push callee-ir52 (make-local-get 27))
        callee-ir54 (vector-push callee-ir53 (make-instr 24 0))
        callee-ir55 (vector-push callee-ir54 (make-local-get 28))
        callee-ir56 (vector-push callee-ir55 (make-instr 24 0))
        callee-ir57 (vector-push callee-ir56 (make-local-get 29))
        callee-ir58 (vector-push callee-ir57 (make-instr 24 0))
        callee-ir59 (vector-push callee-ir58 (make-local-get 30))
        callee-ir60 (vector-push callee-ir59 (make-instr 24 0))
        callee-ir61 (vector-push callee-ir60 (make-local-get 31))
        callee-ir62 (vector-push callee-ir61 (make-instr 24 0))
        callee-ir63 (vector-push callee-ir62 (make-local-get 32))
        callee-ir64 (vector-push callee-ir63 (make-instr 24 0))
        callee-ir65 (vector-push callee-ir64 (make-local-get 33))
        callee-ir66 (vector-push callee-ir65 (make-instr 24 0))
        callee-ir67 (vector-push callee-ir66 (make-local-get 34))
        callee-ir68 (vector-push callee-ir67 (make-instr 24 0))
        callee-ir69 (vector-push callee-ir68 (make-local-get 35))
        callee-ir70 (vector-push callee-ir69 (make-instr 24 0))
        callee-ir71 (vector-push callee-ir70 (make-local-get 36))
        callee-ir72 (vector-push callee-ir71 (make-instr 24 0))
        callee-ir73 (vector-push callee-ir72 (make-local-get 37))
        callee-ir74 (vector-push callee-ir73 (make-instr 24 0))
        callee-ir75 (vector-push callee-ir74 (make-local-get 38))
        callee-ir76 (vector-push callee-ir75 (make-instr 24 0))
        callee-ir77 (vector-push callee-ir76 (make-local-get 39))
        callee-ir78 (vector-push callee-ir77 (make-instr 24 0))
        callee-ir79 (vector-push callee-ir78 (make-local-get 40))
        callee-ir80 (vector-push callee-ir79 (make-instr 24 0))
        callee-ir81 (vector-push callee-ir80 (make-local-get 41))
        callee-ir82 (vector-push callee-ir81 (make-instr 24 0))
        callee-ir83 (vector-push callee-ir82 (make-local-get 42))
        callee-ir84 (vector-push callee-ir83 (make-instr 24 0))
        callee-ir85 (vector-push callee-ir84 (make-local-get 43))
        callee-ir86 (vector-push callee-ir85 (make-instr 24 0))
        callee-ir87 (vector-push callee-ir86 (make-local-get 44))
        callee-ir88 (vector-push callee-ir87 (make-instr 24 0))
        callee-ir89 (vector-push callee-ir88 (make-local-get 45))
        callee-ir90 (vector-push callee-ir89 (make-instr 24 0))
        callee-ir91 (vector-push callee-ir90 (make-local-get 46))
        callee-ir92 (vector-push callee-ir91 (make-instr 24 0))
        callee-ir93 (vector-push callee-ir92 (make-local-get 47))
        callee-ir94 (vector-push callee-ir93 (make-instr 24 0))
        callee-ir callee-ir94
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 48 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_two_arg_drop_restore_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-ir (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push (vector-new 5) (make-instr 3 7))
                          (make-instr 3 40))
                        (make-instr 3 2))
                      (make-call 1))
                    (make-instr 44 0))
        callee-ir (vector-push
                    (vector-push
                      (vector-push (vector-new 3) (make-local-get 0))
                      (make-local-get 1))
                    (make-instr 24 0))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta 2 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_three_value_double_drop_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [ir (vector-push
             (vector-push
               (vector-push
                 (vector-push
                   (vector-push (vector-new 5) (make-instr 3 7))
                   (make-instr 3 40))
                 (make-instr 3 2))
               (make-instr 44 0))
             (make-instr 44 0))
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

fn host_target_direct_call_arg_drop_restore_code_bytes() -> Vec<u8> {
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [caller-with-call (vector-push
                           (vector-push
                             (vector-push
                               (vector-push (vector-new 4) (make-instr 3 7))
                               (make-instr 3 42))
                             (make-call 1))
                           (make-instr 44 0))
        callee-ir (vector-push (vector-new 1) (make-local-get 0))
        caller (make-function-meta 0 0 caller-with-call)
        callee (make-function-meta 1 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

/// ネイティブバイト列を `.s` アセンブリシムでラップし、
/// clang (arm64) でリンクして実行する。戻り値は exit code。
fn link_and_run_native_host_binary(code: &[u8]) -> Result<i32, String> {
    link_and_run_native_host_binary_with_args(code, &[])
}

fn link_and_run_native_host_binary_with_args(code: &[u8], args: &[&str]) -> Result<i32, String> {
    Ok(link_and_run_native_host_binary_capture_with_args(code, args)?.exit_code)
}

fn link_and_run_native_host_binary_capture_with_args(
    code: &[u8],
    args: &[&str],
) -> Result<NativeHostExecutionResult, String> {
    if !host_native_exec_supported() {
        return Err("host native execution は macOS arm64 でのみサポート".to_string());
    }

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-host-exec", id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        // バイト列を _main シンボルのアセンブリ .byte ディレクティブとして書き出す
        let byte_strs: Vec<String> = code.iter().map(|b| format!("0x{b:02x}")).collect();
        let asm_content = format!(
            ".section __TEXT,__text\n\
             .globl _generated\n\
             _generated:\n\
                  .byte {}\n\
             .globl _main\n\
             _main:\n\
                 stp x21, x22, [sp, #-32]!\n\
                 str x30, [sp, #16]\n\
                 mov x19, x0\n\
                 mov x20, x1\n\
                 mov x21, #0\n\
                 mov x22, #0\n\
                 bl _generated\n\
                 ldr x30, [sp, #16]\n\
                 ldp x21, x22, [sp], #32\n\
                 ret\n",
            byte_strs.join(", ")
        );
        std::fs::write(dir.join("prog.s"), &asm_content)
            .map_err(|e| format!("prog.s 書き込み失敗: {e}"))?;

        let link_result = std::process::Command::new("clang")
            .args(["-arch", "arm64", "prog.s", "-o", "prog"])
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("clang 実行失敗: {e}"))?;

        if !link_result.status.success() {
            return Err(format!(
                "リンク失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&link_result.stdout),
                String::from_utf8_lossy(&link_result.stderr),
            ));
        }

        let run_result = std::process::Command::new(dir.join("prog"))
            .args(args)
            .output()
            .map_err(|e| format!("実行失敗: {e}"))?;

        Ok(NativeHostExecutionResult {
            exit_code: run_result.status.code().unwrap_or(-1),
            stdout: run_result.stdout,
            stderr: run_result.stderr,
        })
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn link_and_run_native_host_binary_with_cells_arg(
    code: &[u8],
    cell0: u64,
    cell1: u64,
) -> Result<i32, String> {
    if !host_native_exec_supported() {
        return Err("host native execution は macOS arm64 でのみサポート".to_string());
    }

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-host-exec-cells", id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        let byte_strs: Vec<String> = code.iter().map(|b| format!("0x{b:02x}")).collect();
        let asm_content = format!(
            ".section __DATA,__data\n\
             .p2align 3\n\
             _lsharp_test_cells:\n\
                  .quad {cell0}\n\
                 .quad {cell1}\n\
             .section __TEXT,__text\n\
             .globl _generated\n\
             _generated:\n\
                  .byte {}\n\
             .globl _main\n\
             _main:\n\
                 stp x21, x22, [sp, #-32]!\n\
                 str x30, [sp, #16]\n\
                 mov x19, #0\n\
                 mov x20, #0\n\
                 mov x21, #0\n\
                 mov x22, #0\n\
                 adrp x0, _lsharp_test_cells@PAGE\n\
                 add x0, x0, _lsharp_test_cells@PAGEOFF\n\
                 bl _generated\n\
                 ldr x30, [sp, #16]\n\
                 ldp x21, x22, [sp], #32\n\
                 ret\n",
            byte_strs.join(", ")
        );
        std::fs::write(dir.join("prog.s"), &asm_content)
            .map_err(|e| format!("prog.s 書き込み失敗: {e}"))?;

        let link_result = std::process::Command::new("clang")
            .args(["-arch", "arm64", "prog.s", "-o", "prog"])
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("clang 実行失敗: {e}"))?;

        if !link_result.status.success() {
            return Err(format!(
                "リンク失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&link_result.stdout),
                String::from_utf8_lossy(&link_result.stderr),
            ));
        }

        let run_result = std::process::Command::new(dir.join("prog"))
            .output()
            .map_err(|e| format!("実行失敗: {e}"))?;

        Ok(run_result.status.code().unwrap_or(-1))
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// canonical artifact 名で object / response / binary を materialize し、実行まで行う。
fn materialize_native_host_bundle_artifacts_in_dir(
    dir: &std::path::Path,
    code: &[u8],
) -> Result<String, String> {
    let byte_strs: Vec<String> = code.iter().map(|b| format!("0x{b:02x}")).collect();
    let program_asm = format!(
        ".section __TEXT,__text\n\
             .globl _generated\n\
             _generated:\n\
                  .byte {}\n\
             .globl _main\n\
             _main:\n\
                 stp x21, x22, [sp, #-32]!\n\
                 str x30, [sp, #16]\n\
                 mov x19, x0\n\
                 mov x20, x1\n\
                 mov x21, #0\n\
                 mov x22, #0\n\
                 bl _generated\n\
                 ldr x30, [sp, #16]\n\
                 ldp x21, x22, [sp], #32\n\
                 ret\n",
        byte_strs.join(", ")
    );
    std::fs::write(dir.join("program.s"), program_asm)
        .map_err(|e| format!("program.s 書き込み失敗: {e}"))?;

    let runtime_asm =
        ".section __TEXT,__text\n.globl _lsharp_runtime_stub\n_lsharp_runtime_stub:\n    ret\n";
    std::fs::write(dir.join("runtime.s"), runtime_asm)
        .map_err(|e| format!("runtime.s 書き込み失敗: {e}"))?;

    let compile_program = std::process::Command::new("clang")
        .args(["-arch", "arm64", "-c", "program.s", "-o", "program.o"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("program.o 生成失敗: {e}"))?;
    if !compile_program.status.success() {
        return Err(format!(
            "program.o 生成失敗:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_program.stdout),
            String::from_utf8_lossy(&compile_program.stderr),
        ));
    }

    let compile_runtime = std::process::Command::new("clang")
        .args(["-arch", "arm64", "-c", "runtime.s", "-o", "runtime.o"])
        .current_dir(dir)
        .output()
        .map_err(|e| format!("runtime.o 生成失敗: {e}"))?;
    if !compile_runtime.status.success() {
        return Err(format!(
            "runtime.o 生成失敗:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&compile_runtime.stdout),
            String::from_utf8_lossy(&compile_runtime.stderr),
        ));
    }

    let response_text = "-o\nprogram.native\nprogram.o\nruntime.o\n".to_string();
    std::fs::write(dir.join("linker-response.txt"), &response_text)
        .map_err(|e| format!("linker-response.txt 書き込み失敗: {e}"))?;

    let link_result = std::process::Command::new("clang")
        .arg("-Wl,-stack_size,0x08000000")
        .arg("@linker-response.txt")
        .current_dir(dir)
        .output()
        .map_err(|e| format!("clang response-file 実行失敗: {e}"))?;
    if !link_result.status.success() {
        return Err(format!(
            "canonical response-file link 失敗:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&link_result.stdout),
            String::from_utf8_lossy(&link_result.stderr),
        ));
    }

    let relink_result = std::process::Command::new("clang")
        .arg("-Wl,-stack_size,0x08000000")
        .arg("@linker-response.txt")
        .current_dir(dir)
        .output()
        .map_err(|e| format!("clang stack-size relink 実行失敗: {e}"))?;
    if !relink_result.status.success() {
        return Err(format!(
            "canonical stack-size relink 失敗:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&relink_result.stdout),
            String::from_utf8_lossy(&relink_result.stderr),
        ));
    }

    Ok(response_text)
}

fn read_native_host_bundle_from_dir(
    dir: &std::path::Path,
    response_text: String,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
) -> Result<NativeHostArtifactBundle, String> {
    Ok(NativeHostArtifactBundle {
        program_object: std::fs::read(dir.join("program.o"))
            .map_err(|e| format!("program.o 読み込み失敗: {e}"))?,
        runtime_object: std::fs::read(dir.join("runtime.o"))
            .map_err(|e| format!("runtime.o 読み込み失敗: {e}"))?,
        response_text,
        program_binary: std::fs::read(dir.join("program.native"))
            .map_err(|e| format!("program.native 読み込み失敗: {e}"))?,
        stdout,
        stderr,
        exit_code,
    })
}

fn build_and_run_native_host_bundle_with_canonical_artifacts(
    code: &[u8],
) -> Result<NativeHostArtifactBundle, String> {
    if !host_native_exec_supported() {
        return Err(
            "canonical host bundle materialization は macOS arm64 でのみサポート".to_string(),
        );
    }

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir("e2e-native-fixtures", "native-host-bundle", id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        let response_text = materialize_native_host_bundle_artifacts_in_dir(&dir, code)?;

        let run_result = std::process::Command::new(dir.join("program.native"))
            .output()
            .map_err(|e| format!("program.native 実行失敗: {e}"))?;

        read_native_host_bundle_from_dir(
            &dir,
            response_text,
            run_result.stdout,
            run_result.stderr,
            run_result.status.code().unwrap_or(-1),
        )
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn build_native_host_bundle_with_canonical_artifacts_and_entrypoint(
    code: &[u8],
    entrypoint_offset: usize,
) -> Result<NativeHostArtifactBundle, String> {
    if !host_native_exec_supported() {
        return Err(
            "canonical host bundle materialization は macOS arm64 でのみサポート".to_string(),
        );
    }
    if entrypoint_offset > code.len() {
        return Err(format!(
            "entrypoint offset が code bytes 範囲外: offset={} len={}",
            entrypoint_offset,
            code.len()
        ));
    }
    let entrypoint_offset = align_aarch64_entrypoint_to_function_start(code, entrypoint_offset);

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir(
        "e2e-native-fixtures",
        "native-host-bundle-artifact-entrypoint",
        id,
    );
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        let prefix_bytes = &code[..entrypoint_offset];
        let suffix_bytes = &code[entrypoint_offset..];
        let prefix_text = if prefix_bytes.is_empty() {
            "0x1f, 0x20, 0x03, 0xd5".to_string()
        } else {
            prefix_bytes
                .iter()
                .map(|b| format!("0x{b:02x}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let suffix_text = suffix_bytes
            .iter()
            .map(|b| format!("0x{b:02x}"))
            .collect::<Vec<_>>()
            .join(", ");
        let program_asm = format!(
            ".section __TEXT,__text\n\
             .globl _main\n\
             _main:\n\
                 stp x21, x22, [sp, #-32]!\n\
                 str x30, [sp, #16]\n\
                 mov x19, x0\n\
                 mov x20, x1\n\
                 mov x21, #0\n\
                 mov x22, #0\n\
                 bl _lsharp_entry\n\
                 ldr x30, [sp, #16]\n\
                 ldp x21, x22, [sp], #32\n\
                 ret\n\
             .globl _lsharp_bundle\n\
             _lsharp_bundle:\n\
                  .byte {prefix_text}\n\
             .globl _lsharp_entry\n\
             _lsharp_entry:\n\
                 .byte {suffix_text}\n"
        );
        std::fs::write(dir.join("program.s"), program_asm)
            .map_err(|e| format!("program.s 書き込み失敗: {e}"))?;

        let runtime_asm =
            ".section __TEXT,__text\n.globl _lsharp_runtime_stub\n_lsharp_runtime_stub:\n    ret\n";
        std::fs::write(dir.join("runtime.s"), runtime_asm)
            .map_err(|e| format!("runtime.s 書き込み失敗: {e}"))?;

        let compile_program = std::process::Command::new("clang")
            .args(["-arch", "arm64", "-c", "program.s", "-o", "program.o"])
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("program.o 生成失敗: {e}"))?;
        if !compile_program.status.success() {
            return Err(format!(
                "program.o 生成失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&compile_program.stdout),
                String::from_utf8_lossy(&compile_program.stderr),
            ));
        }

        let compile_runtime = std::process::Command::new("clang")
            .args(["-arch", "arm64", "-c", "runtime.s", "-o", "runtime.o"])
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("runtime.o 生成失敗: {e}"))?;
        if !compile_runtime.status.success() {
            return Err(format!(
                "runtime.o 生成失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&compile_runtime.stdout),
                String::from_utf8_lossy(&compile_runtime.stderr),
            ));
        }

        let response_text = "-o\nprogram.native\nprogram.o\nruntime.o\n".to_string();
        std::fs::write(dir.join("linker-response.txt"), &response_text)
            .map_err(|e| format!("linker-response.txt 書き込み失敗: {e}"))?;

        let link_result = std::process::Command::new("clang")
            .arg("-Wl,-stack_size,0x08000000")
            .arg("@linker-response.txt")
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("clang response-file 実行失敗: {e}"))?;
        if !link_result.status.success() {
            return Err(format!(
                "canonical response-file link 失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&link_result.stdout),
                String::from_utf8_lossy(&link_result.stderr),
            ));
        }

        let relink_result = std::process::Command::new("clang")
            .arg("-Wl,-stack_size,0x08000000")
            .arg("@linker-response.txt")
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("clang stack-size relink 実行失敗: {e}"))?;
        if !relink_result.status.success() {
            return Err(format!(
                "canonical stack-size relink 失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&relink_result.stdout),
                String::from_utf8_lossy(&relink_result.stderr),
            ));
        }

        read_native_host_bundle_from_dir(&dir, response_text, Vec::new(), Vec::new(), 0)
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn build_and_run_native_host_bundle_with_canonical_artifacts_and_entrypoint(
    code: &[u8],
    entrypoint_offset: usize,
) -> Result<NativeHostArtifactBundle, String> {
    if !host_native_exec_supported() {
        return Err(
            "canonical host bundle materialization は macOS arm64 でのみサポート".to_string(),
        );
    }
    if entrypoint_offset > code.len() {
        return Err(format!(
            "entrypoint offset が code bytes 範囲外: offset={} len={}",
            entrypoint_offset,
            code.len()
        ));
    }

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = target_fixture_dir(
        "e2e-native-fixtures",
        "native-host-bundle-artifact-entrypoint-exec",
        id,
    );
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        let bundle = build_native_host_bundle_with_canonical_artifacts_and_entrypoint(
            code,
            entrypoint_offset,
        )?;
        std::fs::write(dir.join("program.o"), &bundle.program_object)
            .map_err(|e| format!("program.o 書き込み失敗: {e}"))?;
        std::fs::write(dir.join("runtime.o"), &bundle.runtime_object)
            .map_err(|e| format!("runtime.o 書き込み失敗: {e}"))?;
        std::fs::write(dir.join("linker-response.txt"), &bundle.response_text)
            .map_err(|e| format!("linker-response.txt 書き込み失敗: {e}"))?;
        let program_binary_path = dir.join("program.native");
        let relink_result = std::process::Command::new("clang")
            .arg("-Wl,-stack_size,0x08000000")
            .arg("@linker-response.txt")
            .current_dir(&dir)
            .output()
            .map_err(|e| format!("final exec-dir relink 実行失敗: {e}"))?;
        if !relink_result.status.success() {
            return Err(format!(
                "final exec-dir relink 失敗:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&relink_result.stdout),
                String::from_utf8_lossy(&relink_result.stderr),
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let permissions = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&program_binary_path, permissions)
                .map_err(|e| format!("program.native の execute bit 設定失敗: {e}"))?;
        }

        let run_result = std::process::Command::new(&program_binary_path)
            .output()
            .map_err(|e| format!("program.native 実行失敗: {e}"))?;

        read_native_host_bundle_from_dir(
            &dir,
            bundle.response_text,
            run_result.stdout,
            run_result.stderr,
            run_result.status.code().unwrap_or(-1),
        )
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn align_aarch64_entrypoint_to_function_start(code: &[u8], entrypoint_offset: usize) -> usize {
    const SAVE_FP_LR_PROLOGUE: [u8; 4] = [0xfd, 0x7b, 0xbf, 0xa9];

    if entrypoint_offset >= code.len() {
        return entrypoint_offset;
    }

    let mut candidate = entrypoint_offset - (entrypoint_offset % 4);
    loop {
        if candidate + SAVE_FP_LR_PROLOGUE.len() <= code.len()
            && code[candidate..candidate + SAVE_FP_LR_PROLOGUE.len()] == SAVE_FP_LR_PROLOGUE
        {
            return candidate;
        }
        if candidate < 4 {
            break;
        }
        candidate -= 4;
    }

    entrypoint_offset
}

#[test]
fn test_align_aarch64_entrypoint_to_function_start_rewinds_to_save_fp_lr_prologue() {
    let code = vec![
        0x1f, 0x20, 0x03, 0xd5, // nop
        0xfd, 0x7b, 0xbf, 0xa9, // stp x29, x30, [sp, #-16]!
        0xff, 0x43, 0x03, 0xd1, // sub sp, sp, #0xd0
        0xe0, 0x03, 0x00, 0xaa, // mov x0, x0
        0xc0, 0x03, 0x5f, 0xd6, // ret
    ];

    assert_eq!(
        align_aarch64_entrypoint_to_function_start(&code, 12),
        4,
        "AArch64 entrypoint は直前の save-fp/lr prologue へ巻き戻すべき"
    );
}

/// NATIVE-HOST-01: stage1-native-emitted object をホスト (aarch64-apple-darwin) でリンク・実行する
///
/// stage1 (Wasm で動く L# コンパイラ) が `host-target()` (aarch64-apple-darwin) 向けに
/// 生成したネイティブコードバイト列を clang (arm64) でリンクし、実行する。
/// 正しい AArch64 コードが生成されていれば exit code 42 が返る。
///
/// RED: NativeCodegen.ls が aarch64 向けに x86_64 コードを生成するため失敗する。
/// GREEN: NativeCodegen.ls に AArch64 命令生成を追加し、aarch64 target で MOVZ W0, #42 + RET が
///        生成されることで exit code 42 を得られる。
#[test]
fn test_e2e_native_host_binary_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_const_42_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: host target 向けコードバイト列が空"
    );

    // ホスト (aarch64-apple-darwin) でリンク・実行して exit code 42 を確認
    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("host binary リンク・実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01b: LocalSet/LocalGet を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_local_roundtrip_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_local_roundtrip_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: LocalSet/LocalGet を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("local roundtrip host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary local roundtrip: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01b1: selfhost-style 1-based param slot を正規化した function-meta bundle が第1引数を返すこと。
#[test]
fn test_e2e_native_host_binary_selfhost_style_param1_bundle_returns_argc() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_style_param1_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost-style param bundle の host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost-style param bundle host binary 実行に失敗");

    assert_eq!(
        exit_code, 1,
        "selfhost-style param bundle: exit code 1 (argc) を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01c0: typed if/else/end を含む plain native code が host 上で true branch を返すこと。
#[test]
fn test_e2e_native_host_binary_if_else_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_if_else_code_bytes(1);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: if/else/end を含む plain native code が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("if/else host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary if/else: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01c1: if-empty/end が false branch で既存 local 値を保つこと。
#[test]
fn test_e2e_native_host_binary_if_empty_false_branch_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_if_empty_local_set_code_bytes(0);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: if-empty/end を含む plain native code が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("if-empty host binary 実行に失敗");

    assert_eq!(
        exit_code,
        7,
        "host binary if-empty false branch: exit code 7 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01c2: block-empty/br/end が block 終端へ分岐できること。
#[test]
fn test_e2e_native_host_binary_block_br_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_block_br_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: block-empty/br/end を含む plain native code が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("block/br host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary block/br: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01c3: typed loop/br_if/end が 1 local の countdown loop を実行できること。
#[test]
fn test_e2e_native_host_binary_loop_br_if_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_loop_countdown_code_bytes(82);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: loop/br_if/end を含む plain native code が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("loop/br_if host binary 実行に失敗");

    assert_eq!(
        exit_code,
        0,
        "host binary loop/br_if: exit code 0 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01c4: loop-empty/br_if/end も同じ 1 local countdown loop を実行できること。
#[test]
fn test_e2e_native_host_binary_loop_empty_br_if_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_loop_countdown_code_bytes(85);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: loop-empty/br_if/end を含む plain native code が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("loop-empty/br_if host binary 実行に失敗");

    assert_eq!(
        exit_code,
        0,
        "host binary loop-empty/br_if: exit code 0 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01c: i32.const/local.get/i32.add を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i32_add_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i32_add_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i32.const/local.get/i32.add を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i32 add host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i32 add: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d: i32.const/local.get/i32.mul を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i32_mul_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i32_mul_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i32.const/local.get/i32.mul を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i32 mul host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i32 mul: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

fn assert_host_target_i32_logic_exit_code(
    name: &str,
    lhs: i32,
    rhs: i32,
    opcode: u32,
    expected_exit_code: i32,
) {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i32_logic_code_bytes(lhs, rhs, opcode);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: {name} を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .unwrap_or_else(|_| panic!("{name} host binary 実行に失敗"));

    assert_eq!(
        exit_code,
        expected_exit_code,
        "host binary {name}: exit code {} を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        expected_exit_code,
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

fn assert_host_target_exit_code(name: &str, code_bytes: Vec<u8>, expected_exit_code: i32) {
    if !host_native_exec_supported() {
        return;
    }

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: {name} を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .unwrap_or_else(|_| panic!("{name} host binary 実行に失敗"));

    assert_eq!(
        exit_code,
        expected_exit_code,
        "host binary {name}: exit code {} を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        expected_exit_code,
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d1: i32.and を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i32_and_link_and_execute() {
    assert_host_target_i32_logic_exit_code("i32 and", 12, 10, 26, 8);
}

/// NATIVE-HOST-01d2: i32.or を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i32_or_link_and_execute() {
    assert_host_target_i32_logic_exit_code("i32 or", 12, 3, 27, 15);
}

/// NATIVE-HOST-01d2a: selfhost builtin logical and (opcode 71) が host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_selfhost_and_link_and_execute() {
    assert_host_target_i32_logic_exit_code("selfhost and", 12, 10, 71, 8);
}

/// NATIVE-HOST-01d2b: selfhost builtin logical or (opcode 72) が host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_selfhost_or_link_and_execute() {
    assert_host_target_i32_logic_exit_code("selfhost or", 12, 3, 72, 15);
}

/// NATIVE-HOST-01d2c: selfhost root_push (opcode 74) が current value を保ったまま drop 後に previous value を復元できること。
#[test]
fn test_e2e_native_host_binary_selfhost_root_push_drop_restores_previous_value() {
    assert_host_target_exit_code(
        "selfhost root_push",
        host_target_selfhost_root_push_drop_restore_code_bytes(),
        7,
    );
}

/// NATIVE-HOST-01d2d: selfhost root_pop (opcode 75) が drop 後に previous value を復元できること。
#[test]
fn test_e2e_native_host_binary_selfhost_root_pop_drop_restores_previous_value() {
    assert_host_target_exit_code(
        "selfhost root_pop",
        host_target_selfhost_root_pop_drop_restore_code_bytes(),
        42,
    );
}

/// NATIVE-HOST-01d2e: selfhost root_set (opcode 76) が bundle path で slot を畳みつつ value を返し、drop 後に bottom value を復元できること。
#[test]
fn test_e2e_native_host_binary_selfhost_root_set_drop_restores_bottom_value() {
    assert_host_target_exit_code(
        "selfhost root_set bundle",
        host_target_selfhost_root_set_drop_restore_bottom_bundle_code_bytes(),
        99,
    );
}

/// NATIVE-HOST-01d2f: selfhost command-line-arg/string-length が no-arg で空文字長 0 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_command_line_arg_string_length_no_arg_returns_zero() {
    assert_host_target_exit_code(
        "selfhost command-line-arg/string-length no-arg bundle",
        host_target_selfhost_command_line_arg_string_length_bundle_code_bytes(1),
        0,
    );
}

/// NATIVE-HOST-01d2g: selfhost command-line-arg/string-length が argv[1] の長さを返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_command_line_arg_string_length_reads_host_argv() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_command_line_arg_string_length_bundle_code_bytes(1);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost command-line-arg/string-length bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &["abc"])
        .expect("selfhost command-line-arg/string-length host binary 実行に失敗");

    assert_eq!(
        exit_code,
        3,
        "host binary selfhost command-line-arg/string-length: exit code 3 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2ga: selfhost string-char-at (opcode 50) が AArch64 bundle path で argv 文字列から 1 byte を読めること。
#[test]
fn test_e2e_native_host_binary_selfhost_command_line_arg_string_char_at_reads_host_argv_byte() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_command_line_arg_string_char_at_bundle_code_bytes(1, 1);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost command-line-arg/string-char-at bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &["abc"])
        .expect("selfhost command-line-arg/string-char-at host binary 実行に失敗");

    assert_eq!(
        exit_code,
        98,
        "host binary selfhost command-line-arg/string-char-at: exit code 98 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2h: selfhost print (opcode 59) が AArch64 bundle path で stdout に整数を出力して 0 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_print_bundle_writes_stdout() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_print_bundle_code_bytes(42);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost print bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost print host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost print bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "42\n",
        "host binary selfhost print bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2i: selfhost vector-new/vector-length が AArch64 bundle path で empty vector の長さ 0 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_vector_new_length_bundle_returns_empty_length() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_vector_new_length_bundle_code_bytes(4);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost vector-new/vector-length bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost vector-new/vector-length host binary 実行に失敗");

    assert_eq!(
        exit_code,
        0,
        "host binary selfhost vector-new/vector-length bundle: exit code 0 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2j: selfhost vector-push (opcode 55) が AArch64 bundle path で spare capacity に 1 要素を追加し、length=1 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_vector_push_bundle_updates_length() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_vector_push_length_bundle_code_bytes(4, 42);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost vector-push/length bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost vector-push/length host binary 実行に失敗");

    assert_eq!(
        exit_code,
        1,
        "host binary selfhost vector-push/length bundle: exit code 1 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2k: selfhost vector-get (opcode 53) が AArch64 bundle path で push 済み要素 42 を読み戻せること。
#[test]
fn test_e2e_native_host_binary_selfhost_vector_get_bundle_reads_pushed_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_vector_push_get_bundle_code_bytes(4, 42, 0);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost vector-push/get bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost vector-push/get host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary selfhost vector-push/get bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2ka: selfhost vector-get (opcode 53) が AArch64 bundle path で manually seeded elem[0]=42 を読み戻せること。
#[test]
fn test_e2e_native_host_binary_selfhost_vector_get_bundle_reads_manual_seeded_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_vector_get_manual_seed_bundle_code_bytes(4, 1, 42, 0);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost vector-get manual-seed bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost vector-get manual-seed host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary selfhost vector-get manual-seed bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2l: selfhost ref-new/ref-get が AArch64 bundle path で作成した ref cell から値 42 を読み戻せること。
#[test]
fn test_e2e_native_host_binary_selfhost_ref_new_get_bundle_reads_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_ref_new_get_bundle_code_bytes(42);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost ref-new/ref-get bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost ref-new/ref-get host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary selfhost ref-new/ref-get bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2m: selfhost ref-set が AArch64 bundle path で ref cell を更新し、drop 後の ref-get が新値 99 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_ref_set_bundle_updates_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_ref_set_get_bundle_code_bytes(10, 99);
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost ref-set/ref-get bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost ref-set/ref-get host binary 実行に失敗");

    assert_eq!(
        exit_code,
        99,
        "host binary selfhost ref-set/ref-get bundle: exit code 99 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2n: selfhost substring (opcode 69) が AArch64 bundle path で "hello"[1,4) の長さ 3 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_substring_bundle_returns_length() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_substring_length_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost substring/string-length bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &["hello"])
        .expect("selfhost substring/string-length host binary 実行に失敗");

    assert_eq!(
        exit_code,
        3,
        "host binary selfhost substring/string-length bundle: exit code 3 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2o: selfhost substring (opcode 69) が AArch64 bundle path で "hello"[1,4) の先頭文字 'e' を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_substring_bundle_copies_payload() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_substring_char_at_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost substring/string-char-at bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &["hello"])
        .expect("selfhost substring/string-char-at host binary 実行に失敗");

    assert_eq!(
        exit_code,
        101,
        "host binary selfhost substring/string-char-at bundle: exit code 101 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2p: selfhost string-concat (opcode 70) が AArch64 bundle path で "ab" + "Z" の長さ 3 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_string_concat_bundle_returns_length() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_string_concat_length_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost string-concat/string-length bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &["ab", "Z"])
        .expect("selfhost string-concat/string-length host binary 実行に失敗");

    assert_eq!(
        exit_code,
        3,
        "host binary selfhost string-concat/string-length bundle: exit code 3 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2q: selfhost string-concat (opcode 70) が AArch64 bundle path で rhs 側の 'Z' を保持できること。
#[test]
fn test_e2e_native_host_binary_selfhost_string_concat_bundle_copies_rhs_payload() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_string_concat_char_at_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost string-concat/string-char-at bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &["ab", "Z"])
        .expect("selfhost string-concat/string-char-at host binary 実行に失敗");

    assert_eq!(
        exit_code,
        90,
        "host binary selfhost string-concat/string-char-at bundle: exit code 90 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2r: selfhost map-new/map-size が AArch64 bundle path で空 map の size 0 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_map_new_size_bundle_returns_zero() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_map_new_size_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost map-new/map-size bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost map-new/map-size host binary 実行に失敗");

    assert_eq!(
        exit_code,
        0,
        "host binary selfhost map-new/map-size bundle: exit code 0 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2w: selfhost map-insert/map-size が AArch64 bundle path で size 1 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_map_insert_size_bundle_returns_one() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_map_insert_size_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost map-insert/map-size bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost map-insert/map-size host binary 実行に失敗");

    assert_eq!(
        exit_code,
        1,
        "host binary selfhost map-insert/map-size bundle: exit code 1 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2x: selfhost map-insert/map-get が AArch64 bundle path で挿入値 42 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_map_insert_get_bundle_returns_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_map_insert_get_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost map-insert/map-get bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost map-insert/map-get host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary selfhost map-insert/map-get bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2x2: selfhost map-insert/map-get の直後に print を続けても AArch64 bundle path の helper offset が崩れず stdout へ 42 を書けること。
#[test]
fn test_e2e_native_host_binary_selfhost_map_insert_get_then_print_writes_stdout() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_map_insert_get_print_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost map-insert/map-get/print bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost map-insert/map-get/print host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost map-insert/map-get/print bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "42\n",
        "host binary selfhost map-insert/map-get/print bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2y: selfhost map-get が AArch64 bundle path で missing key に 0 を返すこと。
#[test]
fn test_e2e_native_host_binary_selfhost_map_get_missing_bundle_returns_zero() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_map_get_missing_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost map-get missing bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("selfhost map-get missing host binary 実行に失敗");

    assert_eq!(
        exit_code,
        0,
        "host binary selfhost map-get missing bundle: exit code 0 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2s: selfhost file-exists? (opcode 73) が AArch64 bundle path で raw path の存在を判定できること。
#[test]
fn test_e2e_native_host_binary_selfhost_file_exists_raw_bundle_returns_one() {
    if !host_native_exec_supported() {
        return;
    }

    let dir = target_fixture_dir(
        "e2e-native-fixtures",
        "native-selfhost-file-exists-raw",
        NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    std::fs::create_dir_all(&dir).expect("file-exists raw fixture dir 作成失敗");
    let path = dir.join("present.txt");
    std::fs::write(&path, "present").expect("file-exists raw fixture file 書き込み失敗");
    let path_arg = path.to_string_lossy().into_owned();

    let code_bytes = host_target_selfhost_file_exists_raw_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost file-exists? raw bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &[&path_arg])
        .expect("selfhost file-exists? raw host binary 実行に失敗");

    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        exit_code,
        1,
        "host binary selfhost file-exists? raw bundle: exit code 1 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2u: selfhost read-file (opcode 64) が AArch64 bundle path で raw path の長さ 5 を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_read_file_raw_bundle_returns_length() {
    if !host_native_exec_supported() {
        return;
    }

    let dir = target_fixture_dir(
        "e2e-native-fixtures",
        "native-selfhost-read-file-raw",
        NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    std::fs::create_dir_all(&dir).expect("read-file raw fixture dir 作成失敗");
    let path = dir.join("fixture.txt");
    std::fs::write(&path, "hello").expect("read-file raw fixture file 書き込み失敗");
    let path_arg = path.to_string_lossy().into_owned();

    let code_bytes = host_target_selfhost_read_file_raw_length_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost read-file raw bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &[&path_arg])
        .expect("selfhost read-file raw host binary 実行に失敗");

    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        exit_code,
        5,
        "host binary selfhost read-file raw bundle: exit code 5 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2t: selfhost file-exists? (opcode 73) が AArch64 bundle path で tagged path の存在を判定できること。
#[test]
fn test_e2e_native_host_binary_selfhost_file_exists_tagged_bundle_returns_one() {
    if !host_native_exec_supported() {
        return;
    }

    let dir = target_fixture_dir(
        "e2e-native-fixtures",
        "native-selfhost-file-exists-tagged",
        NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    std::fs::create_dir_all(&dir).expect("file-exists tagged fixture dir 作成失敗");
    let path = dir.join("present.txt");
    std::fs::write(&path, "present").expect("file-exists tagged fixture file 書き込み失敗");
    let path_arg = path.to_string_lossy().into_owned();

    let code_bytes = host_target_selfhost_file_exists_tagged_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost file-exists? tagged bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &[&path_arg, ""])
        .expect("selfhost file-exists? tagged host binary 実行に失敗");

    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        exit_code,
        1,
        "host binary selfhost file-exists? tagged bundle: exit code 1 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d2v: selfhost read-file (opcode 64) が AArch64 bundle path で tagged path から payload 先頭 byte 'h' を返せること。
#[test]
fn test_e2e_native_host_binary_selfhost_read_file_tagged_bundle_copies_payload() {
    if !host_native_exec_supported() {
        return;
    }

    let dir = target_fixture_dir(
        "e2e-native-fixtures",
        "native-selfhost-read-file-tagged",
        NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed),
    );
    std::fs::create_dir_all(&dir).expect("read-file tagged fixture dir 作成失敗");
    let path = dir.join("fixture.txt");
    std::fs::write(&path, "hello").expect("read-file tagged fixture file 書き込み失敗");
    let path_arg = path.to_string_lossy().into_owned();

    let code_bytes = host_target_selfhost_read_file_tagged_char_at_bundle_code_bytes();
    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost read-file tagged bundle 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary_with_args(&code_bytes, &[&path_arg, ""])
        .expect("selfhost read-file tagged host binary 実行に失敗");

    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        exit_code,
        104,
        "host binary selfhost read-file tagged bundle: exit code 104 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d3: i64.const/local.get/i64.add を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_add_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i64_add_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.const/local.get/i64.add を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i64 add host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 add: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d3: i64.const/local.get/i64.sub を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_sub_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i64_sub_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.const/local.get/i64.sub を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i64 sub host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 sub: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5: i64.const/local.get/i64.mul を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_mul_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i64_mul_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.const/local.get/i64.mul を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i64 mul host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 mul: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5b: i64.const/local.get/i64.div を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_div_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i64_div_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.const/local.get/i64.div を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i64 div host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 div: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5c: i64.const/local.get/i64.rem を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_rem_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i64_rem_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.const/local.get/i64.rem を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("i64 rem host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 rem: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5d: 1 引数 bundle の i64.load offset=8 が host target で実行できること。
#[test]
fn test_e2e_native_host_binary_i64_load_offset_eight_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_memory_bundle_code_bytes(48, 8);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.load bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 0, 42)
        .expect("i64 load host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 load: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5e: 1 引数 bundle の i32.load offset=4 が host target で実行できること。
#[test]
fn test_e2e_native_host_binary_i32_load_offset_four_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_memory_bundle_code_bytes(45, 4);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i32.load bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 42u64 << 32, 0)
        .expect("i32 load host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i32 load: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5f: 1 引数 bundle の i32.load8_u offset=1 が host target で実行できること。
#[test]
fn test_e2e_native_host_binary_i32_load8_u_offset_one_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_memory_bundle_code_bytes(47, 1);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i32.load8_u bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 42u64 << 8, 0)
        .expect("i32 load8_u host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i32 load8_u: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5g: 1 引数 bundle の i64.store offset=8 後に同アドレスを i64.load できること。
#[test]
fn test_e2e_native_host_binary_i64_store_offset_eight_then_load_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes =
        host_target_single_arg_memory_store_then_load_bundle_code_bytes(1, 42, 49, 8, 48, 8);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.store/i64.load bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 0, 0)
        .expect("i64 store/load host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i64 store/load: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5h: 1 引数 bundle の i32.store offset=4 後に同アドレスを i32.load できること。
#[test]
fn test_e2e_native_host_binary_i32_store_offset_four_then_load_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes =
        host_target_single_arg_memory_store_then_load_bundle_code_bytes(3, 42, 46, 4, 45, 4);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i32.store/i32.load bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 0, 0)
        .expect("i32 store/load host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary i32 store/load: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5i: i64.store が top 2 値を消費したあと、spill 側の 2 値を復元できること。
#[test]
fn test_e2e_native_host_binary_i64_store_then_load_double_drop_restores_bottom_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_i64_store_then_load_double_drop_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: i64.store deep restore bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 0, 0)
        .expect("i64 store deep restore host binary 実行に失敗");

    assert_eq!(
        exit_code,
        5,
        "host binary i64 store deep restore: exit code 5 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5j: 1 引数 bundle の memory.fill が複数 byte を埋め、境界外を壊さないこと。
#[test]
fn test_e2e_native_host_binary_memory_fill_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_memory_fill_load_sum_bundle_code_bytes(42, 5);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: memory.fill bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 7u64 << 40, 0)
        .expect("memory.fill host binary 実行に失敗");

    assert_eq!(
        exit_code,
        49,
        "host binary memory.fill: exit code 49 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5k: 1 引数 bundle の memory.copy が複数 byte を複写し、境界外を壊さないこと。
#[test]
fn test_e2e_native_host_binary_memory_copy_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_memory_copy_load_sum_bundle_code_bytes(5);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: memory.copy bundle code bytes が空"
    );

    let dst_cell = 9u64 | (9u64 << 8) | (9u64 << 16) | (9u64 << 24) | (9u64 << 32) | (11u64 << 40);
    let exit_code = link_and_run_native_host_binary_with_cells_arg(
        &code_bytes,
        0x0807_0605_0403_0201,
        dst_cell,
    )
    .expect("memory.copy host binary 実行に失敗");

    assert_eq!(
        exit_code,
        16,
        "host binary memory.copy: exit code 16 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d5l: memory.fill が top 3 値を消費したあと spill 側の底値を復元できること。
#[test]
fn test_e2e_native_host_binary_memory_fill_drop_restores_bottom_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_single_arg_memory_fill_drop_restored_bottom_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: memory.fill deep restore bundle code bytes が空"
    );

    let exit_code = link_and_run_native_host_binary_with_cells_arg(&code_bytes, 0, 0)
        .expect("memory.fill deep restore host binary 実行に失敗");

    assert_eq!(
        exit_code,
        5,
        "host binary memory.fill deep restore: exit code 5 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

fn assert_host_target_i64_compare_exit_code(
    name: &str,
    lhs: i64,
    rhs: i64,
    opcode: u32,
    expected_exit_code: i32,
) {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_i64_compare_code_bytes(lhs, rhs, opcode);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: {name} を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .unwrap_or_else(|_| panic!("{name} host binary 実行に失敗"));

    assert_eq!(
        exit_code,
        expected_exit_code,
        "host binary {name}: exit code {} を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        expected_exit_code,
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01d4: i64.eq を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_eq_link_and_execute() {
    assert_host_target_i64_compare_exit_code("i64 eq", 42, 42, 30, 1);
}

/// NATIVE-HOST-01d5: i64.ne を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_ne_link_and_execute() {
    assert_host_target_i64_compare_exit_code("i64 ne", 42, 2, 31, 1);
}

/// NATIVE-HOST-01d6: i64.lt_s を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_lt_link_and_execute() {
    assert_host_target_i64_compare_exit_code("i64 lt", 2, 42, 32, 1);
}

/// NATIVE-HOST-01d7: i64.gt_s を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_gt_link_and_execute() {
    assert_host_target_i64_compare_exit_code("i64 gt", 42, 2, 33, 1);
}

/// NATIVE-HOST-01d8: i64.le_s を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_le_link_and_execute() {
    assert_host_target_i64_compare_exit_code("i64 le", 42, 42, 34, 1);
}

/// NATIVE-HOST-01d9: i64.ge_s を含む host target バイト列がリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_i64_ge_link_and_execute() {
    assert_host_target_i64_compare_exit_code("i64 ge", 42, 42, 35, 1);
}

/// NATIVE-HOST-01e: Drop が local.get 前の値へ戻せること。
#[test]
fn test_e2e_native_host_binary_drop_restores_previous_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_drop_restore_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: drop restore を含む host target 向けコードバイト列が空"
    );

    let exit_code =
        link_and_run_native_host_binary(&code_bytes).expect("drop restore host binary 実行に失敗");

    assert_eq!(
        exit_code,
        7,
        "host binary drop restore: exit code 7 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01f: function bundle 内の direct call が host target でリンク・実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call bundle を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call bundle host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary direct call bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g: direct call bundle が 1 引数を callee local slot へ渡して実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call arg bundle を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call arg bundle host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary direct call arg bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g2: import prefix を含む actual module index space でも 1 引数 direct call が link/run できること。
#[test]
fn test_e2e_native_host_binary_import_prefixed_direct_call_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_import_prefixed_direct_call_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: import-prefixed direct call arg bundle を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("import-prefixed direct call arg bundle host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary import-prefixed direct call arg bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g3: import boundary call が runtime stub 経由で link/run できること。
#[test]
fn test_e2e_native_host_binary_import_call_stub_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_import_call_stub_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: import call stub を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("import call stub host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary import call stub: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g4: import idx 1 が per-import stub 経由で selfhost __alloc helper に到達し、ret stub の値をそのまま返さないこと。
#[test]
fn test_e2e_native_host_binary_selfhost_alloc_import_bundle_writes_allocated_offset() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_alloc_import_bundle_code_bytes(17);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost alloc import bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost alloc import bundle host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost alloc import bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "65536\n",
        "host binary selfhost alloc import bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g5: alloc-backed i32.store/i32.load が linear offset を x21 heap base に rebased して roundtrip できること。
#[test]
fn test_e2e_native_host_binary_selfhost_alloc_i32_store_load_bundle_writes_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_alloc_i32_store_load_bundle_code_bytes(42);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost alloc i32.store/load bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost alloc i32.store/load bundle host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost alloc i32.store/load bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "42\n",
        "host binary selfhost alloc i32.store/load bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g6: alloc-backed i32.load8_u が rebased address から byte を読めること。
#[test]
fn test_e2e_native_host_binary_selfhost_alloc_i32_load8_u_bundle_writes_byte() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_alloc_i32_load8_u_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost alloc i32.load8_u bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost alloc i32.load8_u bundle host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost alloc i32.load8_u bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "98\n",
        "host binary selfhost alloc i32.load8_u bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g7: alloc-backed i64.store/i64.load が rebased address で roundtrip できること。
#[test]
fn test_e2e_native_host_binary_selfhost_alloc_i64_store_load_bundle_writes_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_alloc_i64_store_load_bundle_code_bytes(42);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost alloc i64.store/load bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost alloc i64.store/load bundle host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost alloc i64.store/load bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "42\n",
        "host binary selfhost alloc i64.store/load bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g8: alloc-backed memory.copy が src/dst offset を x21 heap base へ rebased して byte を複写できること。
#[test]
fn test_e2e_native_host_binary_selfhost_alloc_memory_copy_bundle_writes_copied_byte() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_alloc_memory_copy_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost alloc memory.copy bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost alloc memory.copy bundle host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost alloc memory.copy bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "98\n",
        "host binary selfhost alloc memory.copy bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01g9: alloc-backed memory.fill が rebased destination へ byte を埋められること。
#[test]
fn test_e2e_native_host_binary_selfhost_alloc_memory_fill_bundle_writes_filled_byte() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_selfhost_alloc_memory_fill_bundle_code_bytes(42);

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: selfhost alloc memory.fill bundle 向けコードバイト列が空"
    );

    let result = link_and_run_native_host_binary_capture_with_args(&code_bytes, &[])
        .expect("selfhost alloc memory.fill bundle host binary 実行に失敗");

    assert_eq!(
        result.exit_code,
        0,
        "host binary selfhost alloc memory.fill bundle: exit code 0 を期待したが {} を得た\n\
         stdout={:?}\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        result.exit_code,
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
    assert_eq!(
        String::from_utf8_lossy(&result.stdout),
        "42\n",
        "host binary selfhost alloc memory.fill bundle: stdout が期待値と一致しない\n\
         stderr={:?}\n\
         bytes ({} bytes): {:?}",
        String::from_utf8_lossy(&result.stderr),
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01h: direct call bundle が 2 引数を callee local slots へ渡して実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_two_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_two_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call two-arg bundle を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call two-arg bundle host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary direct call two-arg bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01j: 3 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_three_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_three_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call three-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call three-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        47,
        "host binary direct call three-arg bundle: exit code 47 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01m: 4 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_four_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_four_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call four-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call four-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        54,
        "host binary direct call four-arg bundle: exit code 54 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01n: 5 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_five_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_five_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call five-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call five-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        65,
        "host binary direct call five-arg bundle: exit code 65 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01o: 6 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_six_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_six_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call six-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call six-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        79,
        "host binary direct call six-arg bundle: exit code 79 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01p: 7 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_seven_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_seven_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call seven-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call seven-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        96,
        "host binary direct call seven-arg bundle: exit code 96 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01q: 8 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_eight_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_eight_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call eight-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call eight-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        115,
        "host binary direct call eight-arg bundle: exit code 115 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01r: 9 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_nine_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_nine_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call nine-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call nine-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        138,
        "host binary direct call nine-arg bundle: exit code 138 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01s: 10 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_ten_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_ten_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call ten-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call ten-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        167,
        "host binary direct call ten-arg bundle: exit code 167 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01t: 11 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_eleven_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_eleven_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call eleven-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call eleven-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        198,
        "host binary direct call eleven-arg bundle: exit code 198 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01u: 12 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twelve_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twelve_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twelve-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twelve-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        235,
        "host binary direct call twelve-arg bundle: exit code 235 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01v: 13 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirteen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirteen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirteen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirteen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        248,
        "host binary direct call thirteen-arg bundle: exit code 248 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01w: 14 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fourteen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_fourteen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call fourteen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call fourteen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        242,
        "host binary direct call fourteen-arg bundle: exit code 242 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01x: 15 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifteen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_fifteen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call fifteen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call fifteen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        243,
        "host binary direct call fifteen-arg bundle: exit code 243 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01y: 16 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_sixteen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_sixteen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call sixteen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call sixteen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        245,
        "host binary direct call sixteen-arg bundle: exit code 245 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01z: 17 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_seventeen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_seventeen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call seventeen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call seventeen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        249,
        "host binary direct call seventeen-arg bundle: exit code 249 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01za: 18 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_eighteen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_eighteen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call eighteen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call eighteen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        252,
        "host binary direct call eighteen-arg bundle: exit code 252 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zb: 19 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_nineteen_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_nineteen_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call nineteen-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call nineteen-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        253,
        "host binary direct call nineteen-arg bundle: exit code 253 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zc: 20 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        254,
        "host binary direct call twenty-arg bundle: exit code 254 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zd: 21 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_one_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_one_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-one-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-one-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        255,
        "host binary direct call twenty-one-arg bundle: exit code 255 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01ze: 22 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_two_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_two_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-two-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-two-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        1,
        "host binary direct call twenty-two-arg bundle: exit code 1 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zf: 23 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_three_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_three_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-three-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-three-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary direct call twenty-three-arg bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zg: 24 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_four_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_four_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-four-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-four-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        50,
        "host binary direct call twenty-four-arg bundle: exit code 50 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zh: 25 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_five_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_five_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-five-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-five-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        63,
        "host binary direct call twenty-five-arg bundle: exit code 63 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zi: 26 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_six_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_six_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-six-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-six-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        68,
        "host binary direct call twenty-six-arg bundle: exit code 68 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zj: 27 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_seven_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_seven_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-seven-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-seven-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        75,
        "host binary direct call twenty-seven-arg bundle: exit code 75 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zk: 28 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_eight_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_eight_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-eight-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-eight-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        86,
        "host binary direct call twenty-eight-arg bundle: exit code 86 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zl: 29 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_twenty_nine_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_twenty_nine_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call twenty-nine-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call twenty-nine-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        89,
        "host binary direct call twenty-nine-arg bundle: exit code 89 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zm: 30 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        91,
        "host binary direct call thirty-arg bundle: exit code 91 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zn: 31 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_one_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_one_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-one-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-one-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        95,
        "host binary direct call thirty-one-arg bundle: exit code 95 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zo: 32 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_two_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_two_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-two-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-two-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        101,
        "host binary direct call thirty-two-arg bundle: exit code 101 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zp: 33 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_three_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_three_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-three-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-three-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        111,
        "host binary direct call thirty-three-arg bundle: exit code 111 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zq: 34 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_four_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_four_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-four-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-four-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        123,
        "host binary direct call thirty-four-arg bundle: exit code 123 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zr: 35 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_five_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_five_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-five-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-five-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        136,
        "host binary direct call thirty-five-arg bundle: exit code 136 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zs: 36 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_six_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_six_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-six-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-six-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        150,
        "host binary direct call thirty-six-arg bundle: exit code 150 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zt: 37 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_seven_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_seven_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-seven-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-seven-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        165,
        "host binary direct call thirty-seven-arg bundle: exit code 165 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zu: 38 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_eight_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_eight_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-eight-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-eight-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        181,
        "host binary direct call thirty-eight-arg bundle: exit code 181 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zv: 39 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_thirty_nine_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_thirty_nine_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call thirty-nine-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call thirty-nine-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        198,
        "host binary direct call thirty-nine-arg bundle: exit code 198 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zw: 40 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        216,
        "host binary direct call forty-arg bundle: exit code 216 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zx: 41 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_one_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_one_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-one-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-one-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        235,
        "host binary direct call forty-one-arg bundle: exit code 235 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zy: 42 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_two_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_two_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-two-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-two-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        255,
        "host binary direct call forty-two-arg bundle: exit code 255 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01zz: 43 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_three_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_three_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-three-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-three-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        20,
        "host binary direct call forty-three-arg bundle: exit code 20 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-020a: 44 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_four_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_four_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-four-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-four-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        42,
        "host binary direct call forty-four-arg bundle: exit code 42 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-020b: 45 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_five_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_five_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-five-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-five-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        65,
        "host binary direct call forty-five-arg bundle: exit code 65 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-020c: 46 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_six_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_six_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-six-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-six-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        89,
        "host binary direct call forty-six-arg bundle: exit code 89 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-020d: 47 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_seven_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_seven_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-seven-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-seven-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        114,
        "host binary direct call forty-seven-arg bundle: exit code 114 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-020e: 48 引数 direct call bundle が host binary として link/run できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_eight_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_forty_eight_arg_bundle_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call forty-eight-arg bundle host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call forty-eight-arg host binary 実行に失敗");

    assert_eq!(
        exit_code,
        140,
        "host binary direct call forty-eight-arg bundle: exit code 140 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01k: 2 引数 direct call のあとで one-deeper previous を Drop で取り戻せること。
#[test]
fn test_e2e_native_host_binary_direct_call_two_arg_drop_restores_spilled_previous() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_two_arg_drop_restore_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call two-arg drop restore を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call two-arg drop restore host binary 実行に失敗");

    assert_eq!(
        exit_code,
        7,
        "host binary direct call two-arg drop restore: exit code 7 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01l: 3-value window では drop;drop で spilled previous まで戻れること。
#[test]
fn test_e2e_native_host_binary_three_value_double_drop_restores_bottom_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_three_value_double_drop_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: three-value double-drop host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("three-value double-drop host binary 実行に失敗");

    assert_eq!(
        exit_code,
        7,
        "host binary three-value double-drop: exit code 7 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// NATIVE-HOST-01i: 1 引数 direct call のあとでも Drop が call 前の値へ戻せること。
#[test]
fn test_e2e_native_host_binary_direct_call_arg_drop_restores_previous_value() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_direct_call_arg_drop_restore_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "stage1-native: direct call arg drop restore を含む host target 向けコードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("direct call arg drop restore host binary 実行に失敗");

    assert_eq!(
        exit_code,
        7,
        "host binary direct call arg drop restore: exit code 7 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// V2-08: canonical native artifact bundle で materialize した `program.native` が実行できること。
#[test]
fn test_e2e_native_host_bundle_artifact_writer_materializes_canonical_files() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_const_42_code_bytes();
    let bundle = build_and_run_native_host_bundle_with_canonical_artifacts(&code_bytes)
        .expect("artifact writer 用 canonical bundle materialization に失敗");
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/e2e-native-artifacts")
        .join(format!(
            "native-proxy-artifact-{}",
            NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("artifact writer test dir の作成に失敗");

    write_native_host_bundle_artifact(&dir, "stage2-native", &bundle)
        .expect("native proxy artifact writer に失敗");

    let stage_dir = dir.join("stage2-native");
    assert_eq!(
        std::fs::read(stage_dir.join("program.o")).expect("program.o 読み込み失敗"),
        bundle.program_object,
        "artifact writer が program.o を canonical 名で書き出していない"
    );
    assert_eq!(
        std::fs::read(stage_dir.join("runtime.o")).expect("runtime.o 読み込み失敗"),
        bundle.runtime_object,
        "artifact writer が runtime.o を canonical 名で書き出していない"
    );
    assert_eq!(
        std::fs::read_to_string(stage_dir.join("linker-response.txt"))
            .expect("linker-response.txt 読み込み失敗"),
        bundle.response_text,
        "artifact writer が canonical response file text を保持していない"
    );
    assert_eq!(
        std::fs::read(stage_dir.join("program.native")).expect("program.native 読み込み失敗"),
        bundle.program_binary,
        "artifact writer が program.native を canonical 名で書き出していない"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = std::fs::metadata(stage_dir.join("program.native"))
            .expect("program.native metadata 読み込み失敗")
            .permissions()
            .mode();
        assert!(
            mode & 0o111 != 0,
            "artifact writer が program.native に execute bit を付けていない: mode={mode:o}"
        );
    }
    assert_eq!(
        std::fs::read(stage_dir.join("stdout.txt")).expect("stdout.txt 読み込み失敗"),
        bundle.stdout,
        "artifact writer が stdout.txt を書き出していない"
    );
    assert_eq!(
        std::fs::read(stage_dir.join("stderr.txt")).expect("stderr.txt 読み込み失敗"),
        bundle.stderr,
        "artifact writer が stderr.txt を書き出していない"
    );

    let summary =
        std::fs::read_to_string(stage_dir.join("summary.json")).expect("summary.json 読み込み失敗");
    assert!(
        summary.contains("\"label\":\"stage2-native\""),
        "summary.json に stage label が無い: {summary}"
    );
    assert!(
        summary.contains("\"exit_code\":42"),
        "summary.json に exit_code が無い: {summary}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// V2-08: canonical native artifact bundle で materialize した `program.native` が実行できること。
#[test]
fn test_e2e_native_host_bundle_uses_canonical_artifact_contract() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_const_42_code_bytes();

    let bundle = build_and_run_native_host_bundle_with_canonical_artifacts(&code_bytes)
        .expect("canonical native bundle materialization に失敗");
    maybe_write_native_host_bundle_artifact("stage1-native", &bundle)
        .expect("stage1-native artifact 書き出しに失敗");

    assert_eq!(
        bundle.response_text, "-o\nprogram.native\nprogram.o\nruntime.o\n",
        "canonical linker response text が期待値と一致しない"
    );
    assert!(
        !bundle.program_object.is_empty(),
        "program.o が空 — canonical artifact bundle が materialize できていない"
    );
    assert!(
        !bundle.runtime_object.is_empty(),
        "runtime.o が空 — canonical artifact bundle が materialize できていない"
    );
    assert!(
        !bundle.program_binary.is_empty(),
        "program.native が空 — canonical artifact bundle が materialize できていない"
    );
    assert!(
        bundle.stdout.is_empty(),
        "tiny host-target bundle の stdout は空であるべき"
    );
    assert!(
        bundle.stderr.is_empty(),
        "tiny host-target bundle の stderr は空であるべき"
    );
    assert_eq!(
        bundle.exit_code, 42,
        "canonical artifact bundle から起動した program.native の exit code が 42 でない"
    );
}

/// V2-08: representative build entry 由来の stage1-native artifact が canonical 契約で materialize できること。
#[test]
fn test_e2e_selfhost_main_native_host_bundle_uses_representative_artifact_contract() {
    if !host_native_exec_supported() {
        return;
    }

    let bundle_input = run_selfhost_main_native_function_meta_bundle_host_bytes_harness();
    assert!(
        !bundle_input.code_bytes.is_empty(),
        "representative build entry の native code bytes が空"
    );
    assert!(
        bundle_input.entrypoint_offset < bundle_input.code_bytes.len(),
        "representative entrypoint offset が code bytes 範囲外: offset={} len={} declared_len={} function_starts={} main_func_idx={}",
        bundle_input.entrypoint_offset,
        bundle_input.code_bytes.len(),
        bundle_input.declared_code_len,
        bundle_input.function_start_len,
        bundle_input.main_func_idx
    );

    let bundle = build_native_host_bundle_with_canonical_artifacts_and_entrypoint(
        &bundle_input.code_bytes,
        bundle_input.entrypoint_offset,
    )
    .expect("representative native bundle materialization に失敗");
    maybe_write_native_host_bundle_artifact("stage1-native", &bundle)
        .expect("representative stage1-native artifact 書き出しに失敗");

    assert_eq!(
        bundle.response_text, "-o\nprogram.native\nprogram.o\nruntime.o\n",
        "representative canonical linker response text が期待値と一致しない"
    );
    assert!(
        !bundle.program_object.is_empty(),
        "representative program.o が空"
    );
    assert!(
        !bundle.runtime_object.is_empty(),
        "representative runtime.o が空"
    );
    assert!(
        !bundle.program_binary.is_empty(),
        "representative program.native が空"
    );
    assert!(
        bundle.stdout.is_empty() && bundle.stderr.is_empty() && bundle.exit_code == 0,
        "artifact-only representative bundle は実行情報を持たない想定: stdout={:?} stderr={:?} exit={}",
        bundle.stdout,
        bundle.stderr,
        bundle.exit_code
    );
}

/// V2-08: host-side proxy の `stage2-native` / `stage3-native` 観測面が一致すること。
#[test]
fn test_e2e_stage23_native_host_bundle_proxy_observations_match() {
    if !host_native_exec_supported() {
        return;
    }

    let stage1_code = host_target_const_42_code_bytes();
    let stage2_bundle = build_and_run_native_host_bundle_with_canonical_artifacts(&stage1_code)
        .expect("proxy stage2-native bundle materialization に失敗");
    maybe_write_native_host_bundle_artifact("stage2-native", &stage2_bundle)
        .expect("stage2-native artifact 書き出しに失敗");
    let stage3_code = host_target_const_42_code_bytes();
    let stage3_bundle = build_and_run_native_host_bundle_with_canonical_artifacts(&stage3_code)
        .expect("proxy stage3-native bundle materialization に失敗");
    maybe_write_native_host_bundle_artifact("stage3-native", &stage3_bundle)
        .expect("stage3-native artifact 書き出しに失敗");

    let stage2_obs = observe_native_host_bundle(&stage2_bundle);
    let stage3_obs = observe_native_host_bundle(&stage3_bundle);

    assert_eq!(
        stage2_obs, stage3_obs,
        "host-side proxy stage2/stage3 の exit/stdout/stderr/artifact hash が一致しない"
    );
}

/// V2-08: representative build entry の `program.native` が stage observation を実行できること。
#[test]
fn test_e2e_selfhost_main_representative_native_host_bundle_executes_stage_observation() {
    if !host_native_exec_supported() {
        return;
    }

    let expected_output = compile_and_run_file(&selfhost_main_path());
    let expected = parse_main_stage_observation(&parse_numeric_lines(&expected_output));
    let bundle_input = run_selfhost_main_native_function_meta_bundle_host_bytes_harness();
    let bundle = build_and_run_native_host_bundle_with_canonical_artifacts_and_entrypoint(
        &bundle_input.code_bytes,
        bundle_input.entrypoint_offset,
    )
    .expect("representative native bundle materialization に失敗");

    assert_eq!(
        bundle.exit_code, 0,
        "representative program.native が exit 0 で完走しない: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&bundle.stdout),
        String::from_utf8_lossy(&bundle.stderr)
    );
    assert!(
        bundle.stderr.is_empty(),
        "representative program.native が stderr を出力した: {:?}",
        String::from_utf8_lossy(&bundle.stderr)
    );

    let actual_output = String::from_utf8(bundle.stdout)
        .unwrap_or_else(|e| panic!("representative stdout UTF-8 decode 失敗: {e}"));
    let actual = parse_main_stage_observation(&parse_numeric_lines(&actual_output));
    assert_eq!(
        actual, expected,
        "representative program.native の stage observation が selfhost main と一致しない"
    );
}

/// V2-08: representative build entry 由来の stage2-native / stage3-native artifact 観測面が一致すること。
#[test]
fn test_e2e_stage23_representative_native_host_bundle_artifact_observations_match() {
    if !host_native_exec_supported() {
        return;
    }

    let bundle_input = run_selfhost_main_native_function_meta_bundle_host_bytes_harness();
    let stage2_bundle = build_native_host_bundle_with_canonical_artifacts_and_entrypoint(
        &bundle_input.code_bytes,
        bundle_input.entrypoint_offset,
    )
    .expect("representative stage2-native bundle materialization に失敗");
    maybe_write_native_host_bundle_artifact("stage2-native", &stage2_bundle)
        .expect("representative stage2-native artifact 書き出しに失敗");
    let stage3_bundle = build_native_host_bundle_with_canonical_artifacts_and_entrypoint(
        &bundle_input.code_bytes,
        bundle_input.entrypoint_offset,
    )
    .expect("representative stage3-native bundle materialization に失敗");
    maybe_write_native_host_bundle_artifact("stage3-native", &stage3_bundle)
        .expect("representative stage3-native artifact 書き出しに失敗");

    let stage2_obs = observe_native_host_artifact_bundle(&stage2_bundle);
    let stage3_obs = observe_native_host_artifact_bundle(&stage3_bundle);

    assert_eq!(
        stage2_obs, stage3_obs,
        "representative stage2/stage3 の artifact hash が一致しない"
    );
}

// =============================================================================
// ZERO-DIFF サンプル: Wasm 出力と native exit code の一致検証
//
// 前提: L# の `(defn main [] N)` 相当プログラムにおいて
//   - Wasm パス: `(print N)` → stdout "N"
//   - Native パス: IR {opcode=1, operand=N} → exit code N
// の両者が整数 N で一致することを示す。
// =============================================================================

/// host-target 向けに定数 N を返す AArch64 バイト列を生成して実行し、exit code を返す。
fn native_exit_code_for_const(n: u32) -> i32 {
    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [instr (vector-push (vector-push (vector-new 2) 1) {n})
        ir (vector-push (vector-new 1) instr)
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        n = n
    );
    let code_bytes = run_native_codegen_host_bytes_harness(&source);
    assert!(
        !code_bytes.is_empty(),
        "native const {}: コードバイト列が空",
        n
    );
    link_and_run_native_host_binary(&code_bytes)
        .unwrap_or_else(|e| panic!("native const {}: リンク・実行失敗: {}", n, e))
}

fn native_exit_code_for_const_sequence(values: &[u32]) -> i32 {
    assert!(
        !values.is_empty(),
        "const sequence: 少なくとも 1 個の値が必要"
    );

    let mut bindings = String::new();
    for (idx, value) in values.iter().enumerate() {
        bindings.push_str(&format!(
            "        instr{idx} (vector-push (vector-push (vector-new 2) 3) {value})\n",
        ));
        if idx == 0 {
            bindings.push_str(&format!(
                "        ir0 (vector-push (vector-new {len}) instr0)\n",
                len = values.len()
            ));
        } else {
            bindings.push_str(&format!(
                "        ir{idx} (vector-push ir{prev} instr{idx})\n",
                prev = idx - 1
            ));
        }
    }

    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [
{bindings}        ir ir{last_idx}
        target (host-target)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        bindings = bindings,
        last_idx = values.len() - 1
    );

    let code_bytes = run_native_codegen_host_bytes_harness(&source);
    assert!(
        !code_bytes.is_empty(),
        "native const sequence {:?}: コードバイト列が空",
        values
    );
    link_and_run_native_host_binary(&code_bytes).unwrap_or_else(|e| {
        panic!(
            "native const sequence {:?}: リンク・実行失敗: {}",
            values, e
        )
    })
}

fn native_exit_code_for_const_sequence_after_drops(values: &[u32], drop_count: usize) -> i32 {
    assert!(
        !values.is_empty(),
        "const sequence after drops: 少なくとも 1 個の値が必要"
    );
    assert!(
        drop_count < values.len(),
        "const sequence after drops: drop 回数 {} は値数 {} 未満である必要がある",
        drop_count,
        values.len()
    );

    let mut bindings = String::new();
    for (idx, value) in values.iter().enumerate() {
        bindings.push_str(&format!(
            "        instr{idx} (vector-push (vector-push (vector-new 2) 3) {value})\n",
        ));
        if idx == 0 {
            bindings.push_str(&format!(
                "        ir0 (vector-push (vector-new {len}) instr0)\n",
                len = values.len() + drop_count
            ));
        } else {
            bindings.push_str(&format!(
                "        ir{idx} (vector-push ir{prev} instr{idx})\n",
                prev = idx - 1
            ));
        }
    }

    let mut last_idx = values.len() - 1;
    for drop_idx in 0..drop_count {
        let instr_idx = values.len() + drop_idx;
        bindings.push_str(&format!("        instr{instr_idx} (make-instr 44 0)\n",));
        bindings.push_str(&format!(
            "        ir{instr_idx} (vector-push ir{prev} instr{instr_idx})\n",
            prev = last_idx
        ));
        last_idx = instr_idx;
    }

    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [
{bindings}        ir ir{last_idx}
        func (make-function-meta 0 0 ir)
        functions (vector-push (vector-new 1) func)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        bindings = bindings,
        last_idx = last_idx
    );

    let code_bytes = run_native_codegen_host_bytes_harness(&source);
    assert!(
        !code_bytes.is_empty(),
        "native const sequence {:?} after {} drops: コードバイト列が空",
        values,
        drop_count
    );
    link_and_run_native_host_binary(&code_bytes).unwrap_or_else(|e| {
        panic!(
            "native const sequence {:?} after {} drops: リンク・実行失敗: {}",
            values, drop_count, e
        )
    })
}

fn native_exit_code_for_direct_call_sum(values: &[u32]) -> i32 {
    assert!(
        !values.is_empty(),
        "direct call sum: 少なくとも 1 個の値が必要"
    );

    let mut caller_bindings = String::new();
    for (idx, value) in values.iter().enumerate() {
        if idx == 0 {
            caller_bindings.push_str(&format!(
                "        caller-ir0 (vector-push (vector-new {len}) (make-instr 3 {value}))\n",
                len = values.len() + 1
            ));
        } else {
            caller_bindings.push_str(&format!(
                "        caller-ir{idx} (vector-push caller-ir{prev} (make-instr 3 {value}))\n",
                prev = idx - 1
            ));
        }
    }

    let mut callee_bindings = format!(
        "        callee-ir0 (vector-push (vector-new {len}) (make-local-get 0))\n",
        len = values.len() * 2 - 1
    );
    let mut callee_last = 0usize;
    for idx in 1..values.len() {
        let get_idx = callee_last + 1;
        let add_idx = callee_last + 2;
        callee_bindings.push_str(&format!(
            "        callee-ir{get_idx} (vector-push callee-ir{prev} (make-local-get {idx}))\n",
            prev = callee_last
        ));
        callee_bindings.push_str(&format!(
            "        callee-ir{add_idx} (vector-push callee-ir{get_idx} (make-instr 24 0))\n",
        ));
        callee_last = add_idx;
    }

    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [
{caller_bindings}{callee_bindings}        caller-ir (vector-push caller-ir{caller_last} (make-call 1))
        callee-ir callee-ir{callee_last}
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta {param_count} 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        caller_bindings = caller_bindings,
        callee_bindings = callee_bindings,
        caller_last = values.len() - 1,
        callee_last = callee_last,
        param_count = values.len()
    );

    let code_bytes = run_native_codegen_host_bytes_harness(&source);
    assert!(
        !code_bytes.is_empty(),
        "native direct call sum {:?}: コードバイト列が空",
        values
    );
    link_and_run_native_host_binary(&code_bytes).unwrap_or_else(|e| {
        panic!(
            "native direct call sum {:?}: リンク・実行失敗: {}",
            values, e
        )
    })
}

fn native_exit_code_for_direct_call_local_get(values: &[u32], local_idx: usize) -> i32 {
    assert!(
        !values.is_empty(),
        "direct call local.get: 少なくとも 1 個の値が必要"
    );
    assert!(
        local_idx < values.len(),
        "direct call local.get: local index {} が引数数 {} を超えている",
        local_idx,
        values.len()
    );

    let mut caller_bindings = String::new();
    for (idx, value) in values.iter().enumerate() {
        if idx == 0 {
            caller_bindings.push_str(&format!(
                "        caller-ir0 (vector-push (vector-new {len}) (make-instr 3 {value}))\n",
                len = values.len() + 1
            ));
        } else {
            caller_bindings.push_str(&format!(
                "        caller-ir{idx} (vector-push caller-ir{prev} (make-instr 3 {value}))\n",
                prev = idx - 1
            ));
        }
    }

    let source = format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn make-function-meta [param-count local-count ir]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) param-count)
      local-count)
    ir))

(defn print-bytes [bytes idx len]
  (if (>= idx len)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) len))))

(defn main []
  (let [
{caller_bindings}        caller-ir (vector-push caller-ir{caller_last} (make-call 1))
        callee-ir (vector-push (vector-new 1) (make-local-get {local_idx}))
        caller (make-function-meta 0 0 caller-ir)
        callee (make-function-meta {param_count} 0 callee-ir)
        functions (vector-push (vector-push (vector-new 2) caller) callee)
        target (host-target)
        code (emit-native-function-meta-bundle functions target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        caller_bindings = caller_bindings,
        caller_last = values.len() - 1,
        local_idx = local_idx,
        param_count = values.len()
    );

    let code_bytes = run_native_codegen_host_bytes_harness(&source);
    assert!(
        !code_bytes.is_empty(),
        "native direct call local.get {} for {:?}: コードバイト列が空",
        local_idx,
        values
    );
    link_and_run_native_host_binary(&code_bytes).unwrap_or_else(|e| {
        panic!(
            "native direct call local.get {} for {:?}: リンク・実行失敗: {}",
            local_idx, values, e
        )
    })
}

/// ZERO-DIFF-01: const 0 — Wasm stdout と native exit code がともに 0
#[test]
fn test_e2e_zero_diff_const_0() {
    if !host_native_exec_supported() {
        return;
    }

    let wasm_output = compile_and_run("(defn main [] (do (print 0) 0))");
    assert_eq!(wasm_output.trim(), "0", "Wasm: const 0 を print すること");

    let exit_code = native_exit_code_for_const(0);
    assert_eq!(exit_code, 0, "Native: const 0 → exit code 0");

    assert_eq!(
        wasm_output.trim().parse::<i32>().unwrap(),
        exit_code,
        "ZERO-DIFF: Wasm stdout と native exit code が一致すること (const 0)"
    );
}

/// NATIVE-HOST-020f: 48-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_forty_eight_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    ];
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 26,
        "48-value window の i32.const sequence は最新値 26 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020g: 48 引数 direct call でも最後の引数 local.get 47 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_forty_eight_arg_local_get_47_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    ];
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 47);

    assert_eq!(
        exit_code, 26,
        "48 引数 direct call の local.get 47 は 26 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020h: 48 引数 direct call でも境界直前の local.get 46 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_forty_eight_arg_local_get_46_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    ];
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 46);

    assert_eq!(
        exit_code, 25,
        "48 引数 direct call の local.get 46 は 25 を返すべきだが {} を得た",
        exit_code
    );
}

fn forty_nine_arg_values() -> [u32; 49] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    ]
}

/// NATIVE-HOST-020i: 49 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_forty_nine_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = forty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 167,
        "host binary direct call forty-nine-arg bundle: exit code 167 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020j: 49-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_forty_nine_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = forty_nine_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 27,
        "49-value window の i32.const sequence は最新値 27 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020k: 49 引数 direct call でも末尾 local.get 48 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_forty_nine_arg_local_get_48_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = forty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 48);

    assert_eq!(
        exit_code, 27,
        "49 引数 direct call の local.get 48 は 27 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020l: 49 引数 direct call でも新規 spill 境界の local.get 46 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_forty_nine_arg_local_get_46_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = forty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 46);

    assert_eq!(
        exit_code, 25,
        "49 引数 direct call の local.get 46 は 25 を返すべきだが {} を得た",
        exit_code
    );
}

fn fifty_arg_values() -> [u32; 50] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
    ]
}

/// NATIVE-HOST-020m: 50 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 195,
        "host binary direct call fifty-arg bundle: exit code 195 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020n: 50-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 28,
        "50-value window の i32.const sequence は最新値 28 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020o: 50 引数 direct call でも末尾 local.get 49 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_arg_local_get_49_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 49);

    assert_eq!(
        exit_code, 28,
        "50 引数 direct call の local.get 49 は 28 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020p: 50 引数 direct call でも register spill 境界の local.get 47 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_arg_local_get_47_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 47);

    assert_eq!(
        exit_code, 26,
        "50 引数 direct call の local.get 47 は 26 を返すべきだが {} を得た",
        exit_code
    );
}

fn fifty_one_arg_values() -> [u32; 51] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29,
    ]
}

/// NATIVE-HOST-020q: 51 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_one_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 224,
        "host binary direct call fifty-one-arg bundle: exit code 224 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020r: 51-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_one_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_one_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 29,
        "51-value window の i32.const sequence は最新値 29 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020s: 51 引数 direct call でも末尾 local.get 50 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_one_arg_local_get_50_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 50);

    assert_eq!(
        exit_code, 29,
        "51 引数 direct call の local.get 50 は 29 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020t: 51 引数 direct call でも spill 境界の local.get 48 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_one_arg_local_get_48_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 48);

    assert_eq!(
        exit_code, 27,
        "51 引数 direct call の local.get 48 は 27 を返すべきだが {} を得た",
        exit_code
    );
}

fn fifty_two_arg_values() -> [u32; 52] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30,
    ]
}

/// NATIVE-HOST-020u: 52 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_two_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_two_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 254,
        "host binary direct call fifty-two-arg bundle: exit code 254 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020v: 52-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_two_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_two_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 30,
        "52-value window の i32.const sequence は最新値 30 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020w: 52 引数 direct call でも末尾 local.get 51 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_two_arg_local_get_51_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_two_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 51);

    assert_eq!(
        exit_code, 30,
        "52 引数 direct call の local.get 51 は 30 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020x: 52 引数 direct call でも spill 境界の local.get 49 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_two_arg_local_get_49_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_two_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 49);

    assert_eq!(
        exit_code, 28,
        "52 引数 direct call の local.get 49 は 28 を返すべきだが {} を得た",
        exit_code
    );
}

fn fifty_three_arg_values() -> [u32; 53] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31,
    ]
}

/// NATIVE-HOST-020y: 53 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_three_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_three_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 29,
        "host binary direct call fifty-three-arg bundle: exit code 29 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-020z: 53-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_three_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_three_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 31,
        "53-value window の i32.const sequence は最新値 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021a: 53 引数 direct call でも末尾 local.get 52 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_three_arg_local_get_52_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_three_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 52);

    assert_eq!(
        exit_code, 31,
        "53 引数 direct call の local.get 52 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021b: 53 引数 direct call でも spill 境界の local.get 50 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_three_arg_local_get_50_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_three_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 50);

    assert_eq!(
        exit_code, 29,
        "53 引数 direct call の local.get 50 は 29 を返すべきだが {} を得た",
        exit_code
    );
}

fn fifty_four_arg_values() -> [u32; 54] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32,
    ]
}

fn fifty_five_arg_values() -> [u32; 55] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33,
    ]
}

fn fifty_six_arg_values() -> [u32; 56] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34,
    ]
}

fn fifty_seven_arg_values() -> [u32; 57] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35,
    ]
}

fn fifty_eight_arg_values() -> [u32; 58] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35, 36,
    ]
}

fn fifty_nine_arg_values() -> [u32; 59] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35, 36, 37,
    ]
}

fn sixty_arg_values() -> [u32; 60] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35, 36, 37, 38,
    ]
}

fn sixty_one_arg_values() -> [u32; 61] {
    [
        31, 2, 3, 5, 7, 11, 13, 14, 17, 19, 23, 29, 31, 37, 1, 2, 4, 3, 1, 1, 1, 2, 41, 8, 13, 5,
        7, 11, 3, 2, 4, 6, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    ]
}

/// NATIVE-HOST-021c: 54 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_four_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_four_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 61,
        "host binary direct call fifty-four-arg bundle: exit code 61 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021d: 54-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_four_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_four_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 32,
        "54-value window の i32.const sequence は最新値 32 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021e: 54 引数 direct call でも末尾 local.get 53 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_four_arg_local_get_53_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_four_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 53);

    assert_eq!(
        exit_code, 32,
        "54 引数 direct call の local.get 53 は 32 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ea: 54 引数 direct call でも末尾 1 個手前 local.get 52 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_four_arg_local_get_52_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_four_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 52);

    assert_eq!(
        exit_code, 31,
        "54 引数 direct call の local.get 52 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021f: 54 引数 direct call でも spill 境界の local.get 51 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_four_arg_local_get_51_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_four_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 51);

    assert_eq!(
        exit_code, 30,
        "54 引数 direct call の local.get 51 は 30 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021g: 54 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_four_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_four_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "54 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021h: 55 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_five_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_five_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 94,
        "host binary direct call fifty-five-arg bundle: exit code 94 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021i: 55-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_five_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_five_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 33,
        "55-value window の i32.const sequence は最新値 33 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ia: 55 引数 direct call でも末尾 local.get 54 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_five_arg_local_get_54_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_five_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 54);

    assert_eq!(
        exit_code, 33,
        "55 引数 direct call の local.get 54 は 33 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ib: 55 引数 direct call でも末尾 1 個手前 local.get 53 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_five_arg_local_get_53_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_five_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 53);

    assert_eq!(
        exit_code, 32,
        "55 引数 direct call の local.get 53 は 32 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021j: 55 引数 direct call でも spill 境界の local.get 52 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_five_arg_local_get_52_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_five_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 52);

    assert_eq!(
        exit_code, 31,
        "55 引数 direct call の local.get 52 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021k: 55 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_five_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_five_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "55 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021l: 56 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_six_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_six_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 128,
        "host binary direct call fifty-six-arg bundle: exit code 128 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021m: 56-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_six_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_six_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 34,
        "56-value window の i32.const sequence は最新値 34 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ma: 56 引数 direct call でも末尾 local.get 55 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_six_arg_local_get_55_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_six_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 55);

    assert_eq!(
        exit_code, 34,
        "56 引数 direct call の local.get 55 は 34 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021mb: 56 引数 direct call でも末尾 1 個手前 local.get 54 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_six_arg_local_get_54_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_six_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 54);

    assert_eq!(
        exit_code, 33,
        "56 引数 direct call の local.get 54 は 33 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021n: 56 引数 direct call でも spill 境界の local.get 53 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_six_arg_local_get_53_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_six_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 53);

    assert_eq!(
        exit_code, 32,
        "56 引数 direct call の local.get 53 は 32 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021o: 56 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_six_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_six_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "56 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021p: 57 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_seven_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_seven_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 163,
        "host binary direct call fifty-seven-arg bundle: exit code 163 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021q: 57-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_seven_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_seven_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 35,
        "57-value window の i32.const sequence は最新値 35 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021qa: 57 引数 direct call でも末尾 local.get 56 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_seven_arg_local_get_56_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_seven_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 56);

    assert_eq!(
        exit_code, 35,
        "57 引数 direct call の local.get 56 は 35 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021qb: 57 引数 direct call でも末尾 1 個手前 local.get 55 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_seven_arg_local_get_55_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_seven_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 55);

    assert_eq!(
        exit_code, 34,
        "57 引数 direct call の local.get 55 は 34 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021r: 57 引数 direct call でも spill 境界の local.get 54 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_seven_arg_local_get_54_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_seven_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 54);

    assert_eq!(
        exit_code, 33,
        "57 引数 direct call の local.get 54 は 33 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021s: 57 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_seven_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_seven_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "57 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021t: 58 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_eight_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_eight_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 199,
        "host binary direct call fifty-eight-arg bundle: exit code 199 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021u: 58-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_eight_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_eight_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 36,
        "58-value window の i32.const sequence は最新値 36 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ua: 58 引数 direct call でも末尾 local.get 57 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_eight_arg_local_get_57_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_eight_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 57);

    assert_eq!(
        exit_code, 36,
        "58 引数 direct call の local.get 57 は 36 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ub: 58 引数 direct call でも末尾 1 個手前 local.get 56 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_eight_arg_local_get_56_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_eight_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 56);

    assert_eq!(
        exit_code, 35,
        "58 引数 direct call の local.get 56 は 35 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021v: 58 引数 direct call でも spill 境界の local.get 55 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_eight_arg_local_get_55_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_eight_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 55);

    assert_eq!(
        exit_code, 34,
        "58 引数 direct call の local.get 55 は 34 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021w: 58 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_eight_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_eight_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "58 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021x: 59 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_fifty_nine_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 236,
        "host binary direct call fifty-nine-arg bundle: exit code 236 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021y: 59-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_fifty_nine_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_nine_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 37,
        "59-value window の i32.const sequence は最新値 37 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ya: 59 引数 direct call でも末尾 local.get 58 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_nine_arg_local_get_58_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 58);

    assert_eq!(
        exit_code, 37,
        "59 引数 direct call の local.get 58 は 37 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021yb: 59 引数 direct call でも末尾 1 個手前 local.get 57 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_nine_arg_local_get_57_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 57);

    assert_eq!(
        exit_code, 36,
        "59 引数 direct call の local.get 57 は 36 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021z: 59 引数 direct call でも spill 境界の local.get 56 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_nine_arg_local_get_56_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 56);

    assert_eq!(
        exit_code, 35,
        "59 引数 direct call の local.get 56 は 35 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021za: 59 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_fifty_nine_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = fifty_nine_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "59 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zb: 60 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_sixty_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 18,
        "host binary direct call sixty-arg bundle: exit code 18 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zc: 60-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_sixty_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 38,
        "60-value window の i32.const sequence は最新値 38 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zd: 60 引数 direct call でも末尾 local.get 59 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_arg_local_get_59_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 59);

    assert_eq!(
        exit_code, 38,
        "60 引数 direct call の local.get 59 は 38 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021ze: 60 引数 direct call でも末尾 1 個手前 local.get 58 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_arg_local_get_58_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 58);

    assert_eq!(
        exit_code, 37,
        "60 引数 direct call の local.get 58 は 37 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zf: 60 引数 direct call でも spill 境界の local.get 57 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_arg_local_get_57_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 57);

    assert_eq!(
        exit_code, 36,
        "60 引数 direct call の local.get 57 は 36 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zg: 60 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "60 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zh: 61 引数 direct call bundle でも caller/callee の和を host binary で実行できること。
#[test]
fn test_e2e_native_host_binary_direct_call_sixty_one_arg_bundle_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_sum(&values);

    assert_eq!(
        exit_code, 57,
        "host binary direct call sixty-one-arg bundle: exit code 57 を期待したが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zi: 61-value window の i32.const 連続 push でも最新値を保持できること。
#[test]
fn test_e2e_native_host_binary_sixty_one_i32_const_window_keeps_latest_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_const_sequence(&values);

    assert_eq!(
        exit_code, 39,
        "61-value window の i32.const sequence は最新値 39 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zj: 61-value window で 23 回 drop しても spill 境界の値まで戻れること。
#[test]
fn test_e2e_native_host_binary_sixty_one_i32_const_window_twenty_three_drops_reach_spill21() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_const_sequence_after_drops(&values, 23);

    assert_eq!(
        exit_code, 16,
        "61-value window の 23 drop 後は spill21 の値 16 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zk: 61-value window で 60 回 drop しても最下段の値まで戻れること。
#[test]
fn test_e2e_native_host_binary_sixty_one_i32_const_window_sixty_drops_reach_bottom_value() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_const_sequence_after_drops(&values, 60);

    assert_eq!(
        exit_code, 31,
        "61-value window の 60 drop 後は最下段の値 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zj: 61 引数 direct call でも末尾 local.get 60 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_one_arg_local_get_60_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 60);

    assert_eq!(
        exit_code, 39,
        "61 引数 direct call の local.get 60 は 39 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zk: 61 引数 direct call でも末尾 1 個手前 local.get 59 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_one_arg_local_get_59_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 59);

    assert_eq!(
        exit_code, 38,
        "61 引数 direct call の local.get 59 は 38 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zl: 61 引数 direct call でも spill 境界の local.get 58 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_one_arg_local_get_58_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 58);

    assert_eq!(
        exit_code, 37,
        "61 引数 direct call の local.get 58 は 37 を返すべきだが {} を得た",
        exit_code
    );
}

/// NATIVE-HOST-021zm: 61 引数 direct call でも先頭 local.get 0 を正しく受け取れること。
#[test]
fn test_e2e_native_host_binary_sixty_one_arg_local_get_0_roundtrip() {
    if !host_native_exec_supported() {
        return;
    }

    let values = sixty_one_arg_values();
    let exit_code = native_exit_code_for_direct_call_local_get(&values, 0);

    assert_eq!(
        exit_code, 31,
        "61 引数 direct call の local.get 0 は 31 を返すべきだが {} を得た",
        exit_code
    );
}

/// ZERO-DIFF-02: const 1 — Wasm stdout と native exit code がともに 1
#[test]
fn test_e2e_zero_diff_const_1() {
    if !host_native_exec_supported() {
        return;
    }

    let wasm_output = compile_and_run("(defn main [] (do (print 1) 0))");
    assert_eq!(wasm_output.trim(), "1", "Wasm: const 1 を print すること");

    let exit_code = native_exit_code_for_const(1);
    assert_eq!(exit_code, 1, "Native: const 1 → exit code 1");

    assert_eq!(
        wasm_output.trim().parse::<i32>().unwrap(),
        exit_code,
        "ZERO-DIFF: Wasm stdout と native exit code が一致すること (const 1)"
    );
}

/// ZERO-DIFF-03: const 42 — Wasm stdout と native exit code がともに 42
///
/// `test_e2e_native_host_binary_link_and_execute` で native 側は確認済み。
/// このテストでは Wasm 側も含めた完全な zero-diff を検証する。
#[test]
fn test_e2e_zero_diff_const_42() {
    if !host_native_exec_supported() {
        return;
    }

    let wasm_output = compile_and_run("(defn main [] (do (print 42) 0))");
    assert_eq!(wasm_output.trim(), "42", "Wasm: const 42 を print すること");

    let exit_code = native_exit_code_for_const(42);
    assert_eq!(exit_code, 42, "Native: const 42 → exit code 42");

    assert_eq!(
        wasm_output.trim().parse::<i32>().unwrap(),
        exit_code,
        "ZERO-DIFF: Wasm stdout と native exit code が一致すること (const 42)"
    );
}

/// ZERO-DIFF-04: const 100 — Wasm stdout と native exit code がともに 100
#[test]
fn test_e2e_zero_diff_const_100() {
    if !host_native_exec_supported() {
        return;
    }

    let wasm_output = compile_and_run("(defn main [] (do (print 100) 0))");
    assert_eq!(
        wasm_output.trim(),
        "100",
        "Wasm: const 100 を print すること"
    );

    let exit_code = native_exit_code_for_const(100);
    assert_eq!(exit_code, 100, "Native: const 100 → exit code 100");

    assert_eq!(
        wasm_output.trim().parse::<i32>().unwrap(),
        exit_code,
        "ZERO-DIFF: Wasm stdout と native exit code が一致すること (const 100)"
    );
}

/// ZERO-DIFF-SUMMARY: 代表 4 サンプル (0, 1, 42, 100) の一括 zero-diff レポート
///
/// 各サンプルについて Wasm stdout と native exit code の一致を確認し、
/// zero-diff サンプルの全件合否を出力する。
#[test]
fn test_e2e_zero_diff_sample_summary() {
    if !host_native_exec_supported() {
        return;
    }

    let samples: &[(u32, &str)] = &[(0, "0"), (1, "1"), (42, "42"), (100, "100")];
    let mut passed = 0usize;
    let mut failed_cases: Vec<String> = Vec::new();

    for &(n, expected_str) in samples {
        let wasm_output = compile_and_run(&format!("(defn main [] (do (print {n}) 0))", n = n));
        let wasm_ok = wasm_output.trim() == expected_str;

        let exit_code = native_exit_code_for_const(n);
        let native_ok = exit_code == n as i32;

        let zero_diff_ok =
            wasm_ok && native_ok && wasm_output.trim().parse::<i32>().ok() == Some(exit_code);

        if zero_diff_ok {
            passed += 1;
        } else {
            failed_cases.push(format!(
                "const {n}: wasm={:?} native_exit={exit_code}",
                wasm_output.trim()
            ));
        }
    }

    assert!(
        failed_cases.is_empty(),
        "ZERO-DIFF-SUMMARY: {} / {} サンプルが不一致:\n{}",
        failed_cases.len(),
        samples.len(),
        failed_cases.join("\n")
    );
    assert_eq!(
        passed,
        samples.len(),
        "ZERO-DIFF-SUMMARY: 全 {} サンプルが通過すること",
        samples.len()
    );
}

// =============================================================================
// V2-08-CF: Control-Flow テスト (if/else/end, block/br, loop/brif)
// =============================================================================

fn host_target_if_else_false_code_bytes() -> Vec<u8> {
    // [i32.const 0, if, i32.const 42, else, i32.const 7, end] → else branch → returns 7
    host_target_plain_program_code_bytes(&[(3, 0), (41, 0), (3, 42), (79, 0), (3, 7), (43, 0)])
}

fn x86_target_if_else_code_bytes(cond: i64) -> Vec<u8> {
    // x86_64 target で if/else/end の機械語バイト列を生成する
    run_native_codegen_host_bytes_harness(&format!(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [ic  (make-instr 3 {cond})
        if1 (make-instr 41 0)
        ic42 (make-instr 3 42)
        els (make-instr 79 0)
        ic7 (make-instr 3 7)
        end (make-instr 43 0)
        ir (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push (vector-new 6) ic)
                      if1)
                    ic42)
                  els)
                ic7)
              end)
        target (make-target 1)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
        cond = cond
    ))
}

fn x86_target_loop_brif_code_bytes() -> Vec<u8> {
    // x86_64 target で loop/brif/end の機械語バイト列を生成する
    // IR: [i64.const 1, loop, i64.const 0, brif(0), end]
    // loop opcode=82 (LoopEmpty), brif opcode=81
    run_native_codegen_host_bytes_harness(
        r#"(module Main)
(import Backend.Native.NativeTarget)
(import Backend.Native.NativeCodegen)
(import IR.IR)

(defn print-bytes [bytes idx n]
  (if (>= idx n)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1) n))))

(defn main []
  (let [ic1  (make-instr 1 1)
        lp   (make-instr 82 0)
        ic0  (make-instr 1 0)
        bri  (make-instr 81 0)
        end  (make-instr 43 0)
        ir (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 5) ic1)
                    lp)
                  ic0)
                bri)
              end)
        target (make-target 1)
        code (emit-native ir target)]
    (do
      (print-bytes code 0 (vector-length code))
      0)))"#,
    )
}

/// V2-08-CF-01: if/else/end の false branch が host 上で else 値 (7) を返すこと。
#[test]
fn test_e2e_native_host_binary_if_else_false_branch_link_and_execute() {
    if !host_native_exec_supported() {
        return;
    }

    let code_bytes = host_target_if_else_false_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "V2-08-CF: if/else false branch コードバイト列が空"
    );

    let exit_code = link_and_run_native_host_binary(&code_bytes)
        .expect("if/else false branch host binary 実行に失敗");

    assert_eq!(
        exit_code,
        7,
        "V2-08-CF: if/else false branch: exit code 7 を期待したが {} を得た\n\
         bytes ({} bytes): {:?}",
        exit_code,
        code_bytes.len(),
        code_bytes
    );
}

/// V2-08-CF-BUNDLE-01: function-meta bundle entrypoint path でも if + direct recursion が base case 42 で停止すること。
#[test]
fn test_e2e_native_host_bundle_entrypoint_recursive_if_returns_base_case() {
    if !host_native_exec_supported() {
        return;
    }

    let bundle_input = host_target_recursive_if_bundle_entrypoint();
    assert!(
        !bundle_input.code_bytes.is_empty(),
        "bundle entrypoint recursive if コードバイト列が空"
    );
    assert!(
        bundle_input.entrypoint_offset < bundle_input.code_bytes.len(),
        "bundle entrypoint recursive if の entrypoint offset が範囲外: offset={} len={}",
        bundle_input.entrypoint_offset,
        bundle_input.code_bytes.len()
    );

    let bundle = build_and_run_native_host_bundle_with_canonical_artifacts_and_entrypoint(
        &bundle_input.code_bytes,
        bundle_input.entrypoint_offset,
    )
    .expect("bundle entrypoint recursive if host binary 実行に失敗");

    assert_eq!(
        bundle.exit_code, 42,
        "bundle entrypoint recursive if: exit code 42 を期待したが {} を得た\nstdout={:?}\nstderr={:?}\nbytes ({} bytes): {:?}",
        bundle.exit_code,
        String::from_utf8_lossy(&bundle.stdout),
        String::from_utf8_lossy(&bundle.stderr),
        bundle_input.code_bytes.len(),
        bundle_input.code_bytes
    );
}

/// V2-08-CF-02: x86_64 で if/else/end が TEST+JZ+JMP バイトを含むこと。
#[test]
fn test_native_codegen_emits_x86_if_else_structural_bytes() {
    let code_bytes = x86_target_if_else_code_bytes(1);

    assert!(
        !code_bytes.is_empty(),
        "V2-08-CF: x86_64 if/else コードバイト列が空"
    );

    // TEST eax, eax (85 C0) が含まれることを確認
    let has_test_eax = code_bytes.windows(2).any(|w| w[0] == 0x85 && w[1] == 0xC0);
    assert!(
        has_test_eax,
        "V2-08-CF: x86_64 if/else コードに TEST eax,eax (85 C0) が含まれない\nbytes: {:?}",
        code_bytes
    );

    // JZ rel32 (0F 84) が含まれることを確認
    let has_jz = code_bytes.windows(2).any(|w| w[0] == 0x0F && w[1] == 0x84);
    assert!(
        has_jz,
        "V2-08-CF: x86_64 if/else コードに JZ rel32 (0F 84) が含まれない\nbytes: {:?}",
        code_bytes
    );

    // JMP rel32 (E9) が含まれることを確認 (else → end)
    let has_jmp = code_bytes.iter().any(|&b| b == 0xE9);
    assert!(
        has_jmp,
        "V2-08-CF: x86_64 if/else コードに JMP rel32 (E9) が含まれない\nbytes: {:?}",
        code_bytes
    );

    eprintln!(
        "✓ V2-08-CF: x86_64 if/else structural bytes OK ({} bytes)",
        code_bytes.len()
    );
}

/// V2-08-CF-03: x86_64 で loop/brif/end が JNZ バイトを含むこと。
#[test]
fn test_native_codegen_emits_x86_loop_brif_structural_bytes() {
    let code_bytes = x86_target_loop_brif_code_bytes();

    assert!(
        !code_bytes.is_empty(),
        "V2-08-CF: x86_64 loop/brif コードバイト列が空"
    );

    // TEST eax, eax (85 C0) が含まれることを確認 (brif の条件チェック)
    let has_test_eax = code_bytes.windows(2).any(|w| w[0] == 0x85 && w[1] == 0xC0);
    assert!(
        has_test_eax,
        "V2-08-CF: x86_64 loop/brif コードに TEST eax,eax (85 C0) が含まれない\nbytes: {:?}",
        code_bytes
    );

    // JNZ rel32 (0F 85) が含まれることを確認 (brif の条件分岐)
    let has_jnz = code_bytes.windows(2).any(|w| w[0] == 0x0F && w[1] == 0x85);
    assert!(
        has_jnz,
        "V2-08-CF: x86_64 loop/brif コードに JNZ rel32 (0F 85) が含まれない\nbytes: {:?}",
        code_bytes
    );

    eprintln!(
        "✓ V2-08-CF: x86_64 loop/brif structural bytes OK ({} bytes)",
        code_bytes.len()
    );
}
