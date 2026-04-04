use super::support::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static NATIVE_STAGE_CHAIN_COUNTER: AtomicUsize = AtomicUsize::new(0);
static NATIVE_HOST_EXEC_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, PartialEq, Eq)]
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

    let result = (|| {
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
    })();

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

    let result = (|| {
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
    })();

    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// ネイティブバイト列を `.s` アセンブリシムでラップし、
/// clang (arm64) でリンクして実行する。戻り値は exit code。
fn link_and_run_native_host_binary(code: &[u8]) -> Result<i32, String> {
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
    let code_bytes = run_native_codegen_host_bytes_harness(
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
    );

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
