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
struct NativeHostBundleObservation {
    program_object_hash: u64,
    runtime_object_hash: u64,
    response_text_hash: u64,
    program_binary_hash: u64,
    stdout_hash: u64,
    stderr_hash: u64,
    exit_code: i32,
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
    NativeHostBundleObservation {
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
        stdout_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(&bundle.stdout),
        stderr_hash: super::selfhost_bootstrap_four_layer::hash_fingerprint(&bundle.stderr),
        exit_code: bundle.exit_code,
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
    std::fs::write(stage_dir.join("program.native"), &bundle.program_binary)
        .map_err(|e| format!("program.native 書き込み失敗: {e}"))?;
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

fn run_native_pipeline_harness(entry_source: &str) -> String {
    let id = NATIVE_STAGE_CHAIN_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/e2e-native-fixtures")
        .join(format!("native-stage-chain-{id}"));
    std::fs::create_dir_all(&dir).expect("native stage-chain fixture dir 作成失敗");

    let result = {
        for name in [
            "IR.ls",
            "NativeTarget.ls",
            "NativeCodegen.ls",
            "NativeEmit.ls",
            "Linker.ls",
        ] {
            let path = dir.join(selfhost_fixture_module_relative_path(name));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("native stage-chain parent dir 作成失敗");
            }
            std::fs::write(&path, selfhost_module(name))
                .unwrap_or_else(|_| panic!("{name} 書き込み失敗"));
        }
        std::fs::write(dir.join("Main.ls"), entry_source).expect("Main.ls 書き込み失敗");
        compile_and_run_file(&dir.join("Main.ls"))
    };

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
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/e2e-native-fixtures")
        .join(format!("native-host-bytes-{id}"));
    std::fs::create_dir_all(&dir).expect("native host-bytes fixture dir 作成失敗");

    let result = {
        for name in ["IR.ls", "NativeTarget.ls", "NativeCodegen.ls"] {
            let path = dir.join(selfhost_fixture_module_relative_path(name));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("native host-bytes parent dir 作成失敗");
            }
            std::fs::write(&path, selfhost_module(name))
                .unwrap_or_else(|_| panic!("{name} 書き込み失敗"));
        }
        std::fs::write(dir.join("Main.ls"), entry_source).expect("Main.ls 書き込み失敗");
        let output = compile_and_run_file(&dir.join("Main.ls"));
        output
            .trim()
            .lines()
            .map(|line| {
                line.parse::<u8>()
                    .unwrap_or_else(|_| panic!("byte parse 失敗: {line}"))
            })
            .collect::<Vec<u8>>()
    };

    let _ = std::fs::remove_dir_all(&dir);
    result
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
    if !host_native_exec_supported() {
        return Err("host native execution は macOS arm64 でのみサポート".to_string());
    }

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/e2e-native-fixtures")
        .join(format!("native-host-exec-{id}"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        // バイト列を _main シンボルのアセンブリ .byte ディレクティブとして書き出す
        let byte_strs: Vec<String> = code.iter().map(|b| format!("0x{b:02x}")).collect();
        let asm_content = format!(
            ".section __TEXT,__text\n.globl _main\n_main:\n    .byte {}\n",
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
fn build_and_run_native_host_bundle_with_canonical_artifacts(
    code: &[u8],
) -> Result<NativeHostArtifactBundle, String> {
    if !host_native_exec_supported() {
        return Err(
            "canonical host bundle materialization は macOS arm64 でのみサポート".to_string(),
        );
    }

    let id = NATIVE_HOST_EXEC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/e2e-native-fixtures")
        .join(format!("native-host-bundle-{id}"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let result = (|| {
        let byte_strs: Vec<String> = code.iter().map(|b| format!("0x{b:02x}")).collect();
        let program_asm = format!(
            ".section __TEXT,__text\n.globl _main\n_main:\n    .byte {}\n",
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

        let run_result = std::process::Command::new(dir.join("program.native"))
            .output()
            .map_err(|e| format!("program.native 実行失敗: {e}"))?;

        Ok(NativeHostArtifactBundle {
            program_object: std::fs::read(dir.join("program.o"))
                .map_err(|e| format!("program.o 読み込み失敗: {e}"))?,
            runtime_object: std::fs::read(dir.join("runtime.o"))
                .map_err(|e| format!("runtime.o 読み込み失敗: {e}"))?,
            response_text,
            program_binary: std::fs::read(dir.join("program.native"))
                .map_err(|e| format!("program.native 読み込み失敗: {e}"))?,
            stdout: run_result.stdout,
            stderr: run_result.stderr,
            exit_code: run_result.status.code().unwrap_or(-1),
        })
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
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
