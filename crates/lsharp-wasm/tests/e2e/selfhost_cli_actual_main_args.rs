use super::support::*;
use lsharp_types::intent::review_attestation::{AttestationAlgorithm, ReviewAttestation};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

static CLI_MAIN_ARGS_FIXTURE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn cli_main_args_fixture_dir(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "lsharp_test_cli_main_args_{}_{}_{}",
        prefix,
        std::process::id(),
        CLI_MAIN_ARGS_FIXTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn output_lines(output: String) -> Vec<String> {
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

fn assert_output_lines(lines: &[String], expected: &[&str], message: &str) {
    let actual: Vec<&str> = lines.iter().map(String::as_str).collect();
    assert_eq!(actual, expected, "{}", message);
}

fn doctools_json_snapshot(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/doctools")
        .join(name);
    let snapshot = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("doctools snapshot 読み込み失敗 {}: {}", path.display(), e));
    serde_json::from_str(&snapshot).unwrap_or_else(|e| {
        panic!(
            "doctools snapshot JSON parse 失敗 {}: {}",
            path.display(),
            e
        )
    })
}

fn run_cli_main_with_args(args: &[&str]) -> Vec<String> {
    output_lines(compile_and_run_with_args(
        selfhost_cli_runtime_bundle(),
        args,
    ))
}

fn run_cli_main_with_input_file(prefix: &str, source: &str, args: &[&str]) -> Vec<String> {
    let dir = cli_main_args_fixture_dir(prefix);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source).expect("fixture input.ls の書き込みに失敗");

    let output = compile_and_run_with_dir_and_args(selfhost_cli_runtime_bundle(), &dir, args);
    let _ = std::fs::remove_dir_all(&dir);
    output_lines(output)
}

fn run_cli_main_with_input_file_capture(
    prefix: &str,
    source: &str,
    args: &[&str],
) -> lsharp_wasm::wasi_runner::ExecutionOutput {
    run_main_with_input_file_capture(selfhost_cli_runtime_bundle(), prefix, source, args)
}

fn run_main_with_input_file_capture(
    bundle: &str,
    prefix: &str,
    source: &str,
    args: &[&str],
) -> lsharp_wasm::wasi_runner::ExecutionOutput {
    let dir = cli_main_args_fixture_dir(prefix);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source).expect("fixture input.ls の書き込みに失敗");

    let wasm = compile_only(bundle);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
        &wasm,
        Some(&dir),
        args,
        "",
    )
    .expect("Cli main capture 実行に失敗");

    let _ = std::fs::remove_dir_all(&dir);
    output
}

fn run_compiled_embedded_cli_with_input_file_capture(
    wasm: &[u8],
    prefix: &str,
    source: &str,
    args: &[&str],
) -> (lsharp_wasm::wasi_runner::ExecutionOutput, bool) {
    let dir = cli_main_args_fixture_dir(prefix);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("EmbeddedCli fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source)
        .expect("EmbeddedCli fixture input.ls の書き込みに失敗");

    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
        wasm,
        Some(&dir),
        args,
        "",
    )
    .expect("EmbeddedCli compiled wasm の実行に失敗");
    let manifest_exists = dir.join("intent-graph.json").exists();
    let _ = std::fs::remove_dir_all(&dir);
    (output, manifest_exists)
}

fn run_main_with_input_file_capture_preserve_dir(
    bundle: &str,
    prefix: &str,
    source: &str,
    args: &[&str],
) -> (lsharp_wasm::wasi_runner::ExecutionOutput, PathBuf) {
    let dir = cli_main_args_fixture_dir(prefix);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source).expect("fixture input.ls の書き込みに失敗");

    let wasm = compile_only(bundle);
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
        &wasm,
        Some(&dir),
        args,
        "",
    )
    .expect("Cli main capture 実行に失敗");

    (output, dir)
}

#[test]
fn test_e2e_selfhost_cli_main_compile_and_build_output_actual_preview1_wasm() {
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let wasm = compile_only(selfhost_cli_runtime_bundle());

        for command in ["compile", "build"] {
            let dir = cli_main_args_fixture_dir(&format!("{command}_actual_wasm"));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
            std::fs::write(dir.join("input.ls"), "(defn main [] 42)")
                .expect("fixture input.ls の書き込みに失敗");

            let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                &wasm,
                Some(&dir),
                &[command, "input.ls", "-o", "output.wasm"],
                "",
            )
            .expect("Cli main compile/build 実行に失敗");
            let artifact = std::fs::read(dir.join("output.wasm"))
                .expect("compile/build output artifact の読み込みに失敗");

            assert_eq!(
                output.exit_code, 0,
                "{command} -o は成功終了するべき: stdout={:?}",
                output.stdout
            );
            assert!(
                artifact.starts_with(b"\0asm"),
                "{command} -o は wasm-size summary ではなく actual Preview1 Wasm を書くべき: {:?}",
                artifact
            );
            wasmparser::Validator::new()
                .validate_all(&artifact)
                .unwrap_or_else(|err| {
                    panic!("{command} -o output は valid Wasm であるべき: {err}")
                });
            let execution =
                lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                    &artifact,
                    Some(&dir),
                    &[],
                    "",
                )
                .unwrap_or_else(|err| {
                    panic!("{command} -o output は WASI standalone として実行できるべき: {err}")
                });
            assert_eq!(
                execution.exit_code, 0,
                "{command} -o output は正常終了するべき: stdout={:?}",
                execution.stdout
            );
            assert_eq!(
                execution.stdout, "",
                "{command} -o output は stdout を出さないべき"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }

        let dir = cli_main_args_fixture_dir("component_output_boundary");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
        std::fs::write(dir.join("input.ls"), "(defn main [] 42)")
            .expect("fixture input.ls の書き込みに失敗");
        let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &wasm,
            Some(&dir),
            &[
                "compile",
                "input.ls",
                "--target",
                "wasi-component",
                "-o",
                "output.component.wasm",
            ],
            "",
        )
        .expect("Cli main component output 実行に失敗");
        let output_exists = dir.join("output.component.wasm").exists();
        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(
            output.exit_code, 1,
            "component packaging 未実装時は compile -o を成功扱いにしない: stdout={:?}",
            output.stdout
        );
        assert!(
            !output_exists,
            "component output は external packaging が接続されるまで artifact を書かない"
        );
    });
}

/// TEST-CLI-02-AP: actual Cli main は argv 経由で check file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_check_file() {
    let lines = run_cli_main_with_input_file("check", "(defn main [] 42)", &["check", "input.ls"]);

    assert_output_lines(
        &lines,
        &["Int", "diagnostics:0"],
        "Cli main check argv は型名と diagnostics summary を返すべき",
    );
}

/// EC-M1-03: actual selfhost CLI の check --json が source JSON report を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_check_json_file() {
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "check_json",
            "(defn main [] 42)",
            &["check", "input.ls", "--json"],
        )
    });
    assert_eq!(output.exit_code, 0, "check --json は exit code 0 で終了するべき");
    let lines = output_lines(output.stdout);

    assert_eq!(lines.len(), 1, "actual check --json は JSON report だけを stdout へ返すべき");
    let report: Value =
        serde_json::from_str(&lines[0]).expect("check --json output は valid JSON");
    assert_eq!(report["command"], "check");
    assert_eq!(report["type"], "Int");
    assert_eq!(report["diagnostics"]["count"], 0);
    assert_eq!(report["migration"].as_array().unwrap().len(), 0);
}

/// EC-M1-03: actual selfhost check --json が診断時に exit code 1 を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_check_json_diagnostic_exit() {
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "check_json_diagnostic",
            "(defn main [] (if 42 1 0))",
            &["check", "input.ls", "--json"],
        )
    });
    assert_eq!(
        output.exit_code, 1,
        "診断付き check --json は exit code 1 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(lines.len(), 1, "診断付き actual check --json は report だけを stdout へ返すべき");
    let report: Value =
        serde_json::from_str(&lines[0]).expect("診断付き check --json output は valid JSON");
    assert!(report["diagnostics"]["count"].as_i64().unwrap() > 0);
    assert!(report["diagnostics"]["firstErrorCode"].as_i64().unwrap() > 0);
    assert!(!report["diagnostics"]["message"].as_str().unwrap().is_empty());
}

/// EC-M1-03: actual selfhost check の JSON aliases が同じ report を返すこと
#[test]
fn test_e2e_selfhost_cli_main_check_json_aliases() {
    let reports = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        let wasm = compile_only(selfhost_cli_runtime_bundle());
        let mut reports = Vec::new();
        for (prefix, args) in [
            ("check_json_alias_long", vec!["check", "input.ls", "--json"]),
            (
                "check_json_alias_format",
                vec!["check", "input.ls", "--format", "json"],
            ),
        ] {
            let dir = cli_main_args_fixture_dir(prefix);
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
            std::fs::write(dir.join("input.ls"), "(defn main [] 42)")
                .expect("fixture input.ls の書き込みに失敗");
            let output =
                lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                    &wasm,
                    Some(&dir),
                    &args,
                    "",
                )
                .expect("check JSON alias 実行に失敗");
            let _ = std::fs::remove_dir_all(&dir);
            assert_eq!(output.exit_code, 0, "valid check JSON alias は成功するべき");
            let lines = output_lines(output.stdout);
            assert_eq!(lines.len(), 1, "check JSON alias は report 1 行を返すべき");
            reports.push(
                serde_json::from_str::<Value>(&lines[0])
                    .expect("check JSON alias output は valid JSON であるべき"),
            );
        }
        reports
    });

    assert_eq!(reports[0], reports[1], "--json と --format json は同じ report を返すべき");
    assert_eq!(reports[0]["command"], "check");
    assert_eq!(reports[0]["type"], "Int");
}

/// EC-M1-06: actual selfhost CLI の test --format json が assurance の二軸 report を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_test_format_json_file() {
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "test_format_json",
            "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)",
            &["test", "input.ls", "--format", "json"],
        )
    });
    assert_eq!(
        output.exit_code, 0,
        "test --format json の passing property は exit code 0 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(
        lines.len(),
        1,
        "test --format json は JSON report だけを stdout へ返すべき"
    );
    let report: Value =
        serde_json::from_str(&lines[0]).expect("test --format json output は valid JSON");
    assert!(
        report.get("verified").is_none(),
        "assurance report は top-level verified を返してはならない"
    );
    assert_eq!(report["implementation_conformance"]["status"], "pass");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "sampled-property"
    );
    assert_eq!(report["implementation_conformance"]["cases"], 5);
    assert_eq!(report["implementation_conformance"]["seed"], 0);
    assert_eq!(
        report["implementation_conformance"]["generator"],
        "legacy-deterministic-smoke"
    );
    assert_eq!(
        report["implementation_conformance"]["coverage"]["executed"],
        5
    );
    assert_eq!(report["implementation_conformance"]["target"], "unknown");
    assert_eq!(
        report["implementation_conformance"]["provenance"]["runner"],
        "selfhost"
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
    assert_eq!(report["intent_validation"]["open_questions"], 0);
    assert_eq!(report["intent_validation"]["independent_reviews"], 0);
    assert_eq!(report["intent_validation"]["contradicting_observations"], 0);
}

// Cli / EmbeddedCli の同一 JSON failure contract を共有して検証する。
fn assert_non_bool_invariant_json(output: &lsharp_wasm::wasi_runner::ExecutionOutput) {
    assert_eq!(
        output.exit_code, 2,
        "non-Bool invariant は test --format json を成功扱いせず exit 2 にするべき"
    );
    let lines = output_lines(output.stdout.clone());
    assert_eq!(
        lines.len(),
        1,
        "test --format json の diagnostic failure は JSON report だけを stdout へ返すべき"
    );
    let report: Value = serde_json::from_str(&lines[0])
        .expect("non-Bool invariant の test --format json output は valid JSON であるべき");
    assert!(
        report.get("verified").is_none(),
        "assurance report は top-level verified を返してはならない"
    );
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "legacy-deterministic-smoke"
    );
    assert_eq!(report["implementation_conformance"]["cases"], 0);
    assert_eq!(
        report["implementation_conformance"]["coverage"]["executed"],
        0
    );
    assert_eq!(
        report["implementation_conformance"]["coverage"]["failed"],
        1
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["count"],
        1
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorCode"],
        2
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorSpan"]["start"],
        26
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorSpan"]["end"],
        33
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["runner"],
        "selfhost"
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
}

// Rust checker は期待値を生成する oracle に限定し、実行結果は selfhost 側で検証する。
fn assert_non_bool_invariant_json_matches_rust_oracle(
    output: lsharp_wasm::wasi_runner::ExecutionOutput,
    source: &str,
) {
    assert_non_bool_invariant_json(&output);

    let program = lsharp_syntax::parse(source).expect("oracle fixture は parse できるべき");
    let diagnostic = lsharp_types::metadata_check::check_metadata(&program)
        .into_iter()
        .next()
        .expect("Rust oracle は non-Bool invariant の diagnostic を返すべき");
    assert_eq!(diagnostic.function_name, "succ");

    let report: Value = serde_json::from_str(
        output
            .stdout
            .lines()
            .next()
            .expect("selfhost report は stdout に 1 行を返すべき"),
    )
    .expect("selfhost report は valid JSON であるべき");
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorSpan"]["start"],
        diagnostic.span.start
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorSpan"]["end"],
        diagnostic.span.end
    );
}

/// EC-M1-01/06: actual selfhost CLI の legacy non-Bool invariant が structured report の failure boundary を保つこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_test_format_json_non_bool_invariant() {
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "test_format_json_non_bool_invariant",
            "(defn succ [x] :invariant (+ x 1) (+ x 1))",
            &["test", "input.ls", "--format", "json"],
        )
    });
    assert_non_bool_invariant_json(&output);
}

/// EC-M1-06: EmbeddedCli の実 argv JSON failure が Cli と同じ contract を返すこと
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_non_bool_invariant() {
    let source = "(defn succ [x] :invariant (+ x 1) (+ x 1))";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_json_non_bool_invariant_embedded",
            source,
            &["test", "input.ls", "--format", "json"],
        )
    });
    assert_non_bool_invariant_json_matches_rust_oracle(output, source);
}

/// EC-M1-02/06: EmbeddedCli の property precondition span が Rust oracle と一致すること
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_property_precondition_span() {
    let source = "(defn identity [x] :property [(for-all [x Int] :cases 1 :precondition [(+ x 1)] :postcondition (= result x))] x)";
    let program =
        lsharp_syntax::parse(source).expect("property precondition fixture は parse できるべき");
    let diagnostic = lsharp_types::metadata_check::check_metadata(&program)
        .into_iter()
        .next()
        .expect("Rust oracle は non-Bool precondition の diagnostic を返すべき");
    assert!(
        diagnostic.message.contains("Bool"),
        "Rust oracle は property precondition の Bool 契約を診断するべき: {diagnostic:?}"
    );

    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_json_property_precondition_embedded",
            source,
            &["test", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の property precondition diagnostic は exit 2 を返すべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(
        lines.len(),
        1,
        "EmbeddedCli の test --format json は report だけを stdout へ返すべき"
    );
    let report: Value = serde_json::from_str(&lines[0])
        .expect("EmbeddedCli の property precondition report は valid JSON であるべき");
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorCode"],
        2
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorSpan"]["start"],
        diagnostic.span.start
    );
    assert_eq!(
        report["implementation_conformance"]["diagnostics"]["firstErrorSpan"]["end"],
        diagnostic.span.end
    );
}

/// EC-M2-03: EmbeddedCli の validate source/report/exit contract は Cli と同じ unknown を返す。
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_validate_source_json_trace_gap() {
    let source =
        "(defn cancel [] :intent \"intent:checkout/safe-cancel\" \"Users can cancel\" true)";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "validate_source_trace_gap_embedded",
            source,
            &["validate", "--source", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の未接続 intent は unknown exit 2 で返すべき: stdout={:?}",
        output.stdout
    );
    let lines: Vec<&str> = output.stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "validate report は stdout 1 行の JSON であるべき"
    );
    let report: Value = serde_json::from_str(lines[0]).expect("validate report は valid JSON");
    assert_eq!(report["status"], "unknown");
    assert_eq!(
        report["trace_gaps"][0]["code"],
        "trace-gap.intent-without-claim"
    );
    assert_eq!(
        report["trace_gaps"][0]["subject_id"],
        "intent:checkout/safe-cancel"
    );
    assert_eq!(report["open_questions"], 0);
    assert_eq!(report["independent_reviews"], 0);
    assert_eq!(report["contradicting_observations"], 0);
    assert!(report.get("verified").is_none());
}

/// EC-M2-03: EmbeddedCli の text report は Rust oracle と同じ deterministic projection を返す。
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_validate_source_text_trace_gap() {
    let source =
        "(defn cancel [] :intent \"intent:checkout/safe-cancel\" \"Users can cancel\" true)";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "validate_source_text_trace_gap_embedded",
            source,
            &["validate", "--source", "input.ls", "--format", "text"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の未接続 intent は unknown exit 2 で返すべき: stdout={:?}",
        output.stdout
    );
    assert_eq!(
        output.stdout.trim_end(),
        "status: unknown\n\
trace-gap.intent-without-claim: intent:checkout/safe-cancel\n\
open-questions: 0\n\
independent-reviews: 0\n\
contradicting-observations: 0\n\
stale-reviews: 0\n\
stale-evidence: 0",
        "EmbeddedCli validate --source --format text は deterministic report を返すべき"
    );
    assert!(!output.stdout.contains("{"));
    assert!(!output.stdout.contains("verified"));
}

/// EC-M2-03: EmbeddedCli の complete validation graph は text でも pass を返す。
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_validate_source_text_pass() {
    let source = r#"
(defn verify []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "pass"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-25T00:00:00Z"
    :independence "independent-review"
  :supports "evidence:checkout/review" "claim:checkout/rejects"
  true)
"#;
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "validate_source_text_pass_embedded",
            source,
            &["validate", "--source", "input.ls", "--format", "text"],
        )
    });

    assert_eq!(
        output.exit_code, 0,
        "complete graph with independent review は pass exit 0 であるべき: stdout={:?}",
        output.stdout
    );
    assert_eq!(
        output.stdout.trim_end(),
        "status: pass\n\
open-questions: 0\n\
independent-reviews: 1\n\
contradicting-observations: 0\n\
stale-reviews: 0\n\
stale-evidence: 0",
        "EmbeddedCli の pass text report は Rust ValidationReport::to_text と一致するべき"
    );
    assert!(!output.stdout.contains("verified"));
}

/// EC-M2-02/EC-M3-03: EmbeddedCli の source sampling error は report/manifest を残さず fail-closed にする。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_rejects_negative_sampling_without_report_or_manifest() {
    let source = r#"
(defn invalid []
  :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
  :evidence "evidence:checkout/negative-cases"
    :subject "claim:checkout/cancel-rejects-shipped"
    :method "property"
    :outcome "pass"
    :runner "source-negative-cases"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-cases-commit"
    :artifact-digest "sha256:source-negative-cases"
    :cases -1
    :seed 0
    :generator "source-negative-cases-generator"
    :producer "source-negative-cases-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
"#;
    let (output, dir) = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture_preserve_dir(
            selfhost_embedded_cli_runtime_bundle(),
            "embedded_validate_negative_sampling",
            source,
            &[
                "validate",
                "--source",
                "input.ls",
                "--format",
                "json",
                "--emit-manifest",
                "intent-graph.json",
            ],
        )
    });
    let manifest_exists = dir.join("intent-graph.json").exists();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        output.exit_code, 1,
        "EmbeddedCli の invalid sampling は exit code 1 で拒否するべき: stdout={:?}",
        output.stdout
    );
    assert!(
        output.stdout.contains("source validation error:11"),
        "EmbeddedCli は invalid sampling の入力診断を出すべき: {}",
        output.stdout
    );
    assert!(
        !output.stdout.contains("\"status\""),
        "EmbeddedCli は invalid sampling の report を出力しないべき: {}",
        output.stdout
    );
    assert!(!manifest_exists, "EmbeddedCli は invalid sampling で manifest を残さないべき");
}

/// EC-M2-02/EC-M3-03: EmbeddedCli の seed/shrinks sampling error も report/manifest を残さず fail-closed にする。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_rejects_negative_seed_and_shrinks_without_report_or_manifest() {
    let sources = [
        (
            "negative_seed",
            r#"
(defn invalid []
  :claim "claim:checkout/negative-seed" "The API rejects negative seed"
  :evidence "evidence:checkout/negative-seed"
    :subject "claim:checkout/negative-seed"
    :method "property"
    :outcome "pass"
    :runner "source-negative-seed"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-seed-commit"
    :artifact-digest "sha256:source-negative-seed"
    :cases 1
    :seed -1
    :generator "source-negative-seed-generator"
    :producer "source-negative-seed-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
"#,
        ),
        (
            "negative_shrinks",
            r#"
(defn invalid []
  :claim "claim:checkout/negative-shrinks" "The API rejects negative shrinks"
  :evidence "evidence:checkout/negative-shrinks"
    :subject "claim:checkout/negative-shrinks"
    :method "property"
    :outcome "pass"
    :runner "source-negative-shrinks"
    :target "aarch64-apple-darwin"
    :source-commit "source-negative-shrinks-commit"
    :artifact-digest "sha256:source-negative-shrinks"
    :cases 1
    :seed 0
    :generator "source-negative-shrinks-generator"
    :shrinks [-1]
    :producer "source-negative-shrinks-producer"
    :tool-version "0.2.0-dev"
    :timestamp "2026-07-29T00:00:00Z"
    :independence "same-author"
  true)
"#,
        ),
    ];

    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        let wasm = compile_only(selfhost_embedded_cli_runtime_bundle());
        for (field, source) in sources {
            let (output, manifest_exists) = run_compiled_embedded_cli_with_input_file_capture(
                &wasm,
                &format!("validate_negative_{field}"),
                source,
                &[
                    "validate",
                    "--source",
                    "input.ls",
                    "--format",
                    "json",
                    "--emit-manifest",
                    "intent-graph.json",
                ],
            );
            assert_eq!(
                output.exit_code, 1,
                "EmbeddedCli の negative {field} は exit code 1 で拒否するべき: stdout={:?}",
                output.stdout
            );
            assert!(
                output.stdout.contains("source validation error:11"),
                "EmbeddedCli は negative {field} の入力診断を出すべき: {}",
                output.stdout
            );
            assert!(
                !output.stdout.contains("\"status\""),
                "EmbeddedCli は negative {field} の report を出力しないべき: {}",
                output.stdout
            );
            assert!(
                !manifest_exists,
                "EmbeddedCli は negative {field} で manifest を残さないべき"
            );
        }
    });
}

/// EC-M3-03: EmbeddedCli は validate source の report と manifest を同時に出力すること
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_emits_manifest() {
    let source =
        r#"(defn verify [] :claim "claim:checkout/rejects" "Shipped orders are rejected" true)"#;
    let (output, dir) = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture_preserve_dir(
            selfhost_embedded_cli_runtime_bundle(),
            "embedded_validate_manifest_boundary",
            source,
            &[
                "validate",
                "--source",
                "input.ls",
                "--format",
                "json",
                "--emit-manifest",
                "intent-graph.json",
            ],
        )
    });
    let manifest = std::fs::read_to_string(dir.join("intent-graph.json"))
        .expect("EmbeddedCli は manifest を出力するべき");
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        output.exit_code, 2,
        "manifest 出力後も unknown は exit 2 を返すべき"
    );
    let lines: Vec<&str> = output.stdout.trim().lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "validate report は stdout 1 行の JSON であるべき"
    );
    let report: Value = serde_json::from_str(lines[0]).expect("validate report は valid JSON");
    assert_eq!(report["status"], "unknown");
    assert!(
        !output
            .stdout
            .contains("external-boundary:embedded-cli-manifest-output")
    );

    let manifest: Value = serde_json::from_str(&manifest).expect("manifest は valid JSON");
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["nodes"][0]["kind"], "claim");
    assert_eq!(manifest["nodes"][0]["namespace"], "checkout");
    assert_eq!(manifest["nodes"][0]["key"], "rejects");
    assert!(manifest["evidence"].as_array().unwrap().is_empty());
    assert!(manifest["edges"].as_array().unwrap().is_empty());
}

/// EC-M3-03: EmbeddedCli の validation は独立 review を含む complete graph を pass にすること
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_reports_pass() {
    let source = r#"
(defn verify []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "pass"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-25T00:00:00Z"
    :independence "independent-review"
  :supports "evidence:checkout/review" "claim:checkout/rejects"
  true)
"#;
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "embedded_validate_pass",
            source,
            &["validate", "--source", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 0,
        "complete graph with independent review は pass exit 0 であるべき: stdout={:?}",
        output.stdout
    );
    let report: Value = serde_json::from_str(output.stdout.trim())
        .expect("EmbeddedCli pass report は valid JSON であるべき");
    assert_eq!(report["status"], "pass");
    assert_eq!(report["trace_gaps"].as_array().unwrap().len(), 0);
    assert_eq!(report["open_questions"], 0);
    assert_eq!(report["independent_reviews"], 1);
    assert_eq!(report["contradicting_observations"], 0);
}

/// EC-M3-04: EmbeddedCli は source attestation を unverified fact と manifest state へ投影する。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_projects_review_attestation() {
    let source = r#"
(defn verify []
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :review "review:checkout/reviewer-002" "sha256:review-002" "redacted"
  :review "review:checkout/reviewer-001" "sha256:review-001" "redacted"
  :review-attestation
    :review-id "review:checkout/reviewer-002"
    :subject-digest "sha256:subject-001"
    :source-commit "0123456789abcdef"
    :provenance-digest "sha256:review-002"
    :provider "github"
    :key-id "org/reviews-2026"
    :algorithm "ed25519"
    :signature "AAECAw"
    :issued-at "2026-08-01T00:00:00Z"
    :expires-at "2026-09-01T00:00:00Z"
    :sequence 3
  :review-attestation
    :review-id "review:checkout/reviewer-001"
    :subject-digest "sha256:subject-001"
    :source-commit "0123456789abcdef"
    :provenance-digest "sha256:review-001"
    :provider "github"
    :key-id "org/reviews-2026"
    :algorithm "ed25519"
    :signature "AAECAw"
    :issued-at "2026-08-01T00:00:00Z"
    :expires-at "2026-09-01T00:00:00Z"
    :sequence 3
    true)
"#;
    let (output, text_output, manifest) =
        run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
            let wasm = compile_only(selfhost_embedded_cli_runtime_bundle());
            let dir = cli_main_args_fixture_dir("embedded_validate_review_attestation");
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir)
                .expect("review attestation fixture directory の作成に失敗");
            std::fs::write(dir.join("input.ls"), source)
                .expect("review attestation fixture input.ls の書き込みに失敗");
            let output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
                &wasm,
                Some(&dir),
                &[
                    "validate",
                    "--source",
                    "input.ls",
                    "--format",
                    "json",
                    "--emit-manifest",
                    "intent-graph.json",
                ],
                "",
            )
            .expect("EmbeddedCli review attestation JSON 実行に失敗");
            let manifest = std::fs::read_to_string(dir.join("intent-graph.json"))
                .expect("review attestation の manifest を出力するべき");
            let _ = std::fs::remove_dir_all(&dir);
            let (text_output, _) = run_compiled_embedded_cli_with_input_file_capture(
                &wasm,
                "embedded_validate_review_attestation_text",
                source,
                &["validate", "--source", "input.ls", "--format", "text"],
            );
            (output, text_output, manifest)
        });

    assert_eq!(
        output.exit_code, 2,
        "source attestation 単体は unverified で unknown になるべき"
    );
    let report: Value = serde_json::from_str(output.stdout.trim())
        .expect("EmbeddedCli review attestation report は valid JSON であるべき");
    assert_eq!(
        report["review_verifications"],
        serde_json::json!([
            {
                "review_id": "review:checkout/reviewer-001",
                "state": "unverified"
            },
            {
                "review_id": "review:checkout/reviewer-002",
                "state": "unverified"
            }
        ])
    );
    let expected_canonical = ReviewAttestation::new(
        "review:checkout/reviewer-001",
        "sha256:subject-001",
        "0123456789abcdef",
        "sha256:review-001",
        "github",
        "org/reviews-2026",
        AttestationAlgorithm::Ed25519,
        "2026-08-01T00:00:00Z",
        Some("2026-09-01T00:00:00Z".to_string()),
        3,
        vec![0, 1, 2],
    )
    .expect("review attestation projection fixture は valid であるべき")
    .canonical_bytes();
    let projections = report["review_attestations"]
        .as_array()
        .expect("review_attestations は array であるべき");
    assert_eq!(projections.len(), 2);
    assert_eq!(projections[0]["review_id"], "review:checkout/reviewer-001");
    assert_eq!(projections[0]["subject_digest"], "sha256:subject-001");
    assert_eq!(projections[0]["source_commit"], "0123456789abcdef");
    assert_eq!(projections[0]["provenance_digest"], "sha256:review-001");
    assert_eq!(projections[0]["provider"], "github");
    assert_eq!(projections[0]["key_id"], "org/reviews-2026");
    assert_eq!(projections[0]["algorithm"], "ed25519");
    assert_eq!(projections[0]["signature"], "AAECAw");
    assert_eq!(projections[0]["issued_at"], "2026-08-01T00:00:00Z");
    assert_eq!(projections[0]["expires_at"], "2026-09-01T00:00:00Z");
    assert_eq!(projections[0]["sequence"], 3);
    assert_eq!(projections[0]["state"], "unverified");
    assert_eq!(
        projections[0]["canonical_bytes"],
        serde_json::to_value(expected_canonical).expect("canonical bytes は JSON 化できるべき")
    );
    assert!(
        projections[0]["span"]["start"].as_u64().unwrap()
            < projections[0]["span"]["end"].as_u64().unwrap()
    );
    assert_eq!(projections[1]["review_id"], "review:checkout/reviewer-002");
    assert_eq!(projections[1]["provenance_digest"], "sha256:review-002");
    assert_eq!(projections[1]["state"], "unverified");

    let manifest: Value =
        serde_json::from_str(&manifest).expect("manifest は valid JSON であるべき");
    assert_eq!(manifest["reviews"].as_array().unwrap().len(), 2);
    assert!(manifest["reviews"]
        .as_array()
        .unwrap()
        .iter()
        .all(|review| review["verification_state"] == "unverified"));
    assert_eq!(text_output.exit_code, 2);
    let first = text_output
        .stdout
        .find("review-verification: review:checkout/reviewer-001=unverified")
        .expect("text report は最初の review verification を返すべき");
    let second = text_output
        .stdout
        .find("review-verification: review:checkout/reviewer-002=unverified")
        .expect("text report は二つ目の review verification を返すべき");
    assert!(
        first < second,
        "text report は review_id の決定順を保つべき"
    );
}

/// EC-M3-05: EmbeddedCli は明示した review evidence identity を report/manifest へ投影する。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_projects_explicit_review_evidence_identity() {
    let source = r#"
(defn review []
  :review "review:checkout/reviewer-001" "sha256:review-001" "redacted"
  true)
"#;
    let args = [
        "validate",
        "--source",
        "input.ls",
        "--format",
        "json",
        "--emit-manifest",
        "intent-graph.json",
        "--review-subject-digest",
        "sha256:graph",
        "--review-source-commit",
        "commit-1",
        "--review-artifact-digest",
        "sha256:artifact",
        "--review-trust-store-digest",
        "sha256:trust",
        "--review-lifecycle-digest",
        "sha256:lifecycle",
        "--review-now",
        "2026-08-15T00:00:00Z",
    ];
    let (output, dir) = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        run_main_with_input_file_capture_preserve_dir(
            selfhost_embedded_cli_runtime_bundle(),
            "embedded_validate_review_evidence_identity",
            source,
            &args,
        )
    });
    let manifest = std::fs::read_to_string(dir.join("intent-graph.json"));
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        output.exit_code, 2,
        "identity を付けた未検証 review は unknown のままにするべき: stdout={:?}",
        output.stdout
    );
    let report: Value = serde_json::from_str(output.stdout.trim())
        .expect("explicit identity report は valid JSON であるべき");
    assert_eq!(
        report["review_evidence_identity"],
        serde_json::json!({
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": "sha256:trust",
            "lifecycle_digest": "sha256:lifecycle",
            "now": "2026-08-15T00:00:00Z"
        })
    );
    let manifest: Value = serde_json::from_str(
        &manifest.expect("explicit identity manifest は出力されるべき"),
    )
    .expect("explicit identity manifest は valid JSON であるべき");
    assert_eq!(
        manifest["review_evidence_identity"],
        report["review_evidence_identity"]
    );
}

/// EC-M3-05-N1: 通常の selfhost App.Cli も EmbeddedCli と同じ review evidence identity を
/// JSON/text/manifest の全出力へ投影すること。
#[test]
fn test_e2e_selfhost_cli_main_validate_projects_explicit_review_evidence_identity() {
    let source = r#"
(defn review []
  :review "review:checkout/reviewer-001" "sha256:review-001" "redacted"
  true)
"#;
    run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        let wasm = compile_only(selfhost_cli_runtime_bundle());
        let identity_args = [
            "validate",
            "--source",
            "input.ls",
            "--format",
            "json",
            "--emit-manifest",
            "intent-graph.json",
            "--review-subject-digest",
            "sha256:graph",
            "--review-source-commit",
            "commit-1",
            "--review-artifact-digest",
            "sha256:artifact",
            "--review-trust-store-digest",
            "sha256:trust",
            "--review-lifecycle-digest",
            "sha256:lifecycle",
            "--review-now",
            "2026-08-15T00:00:00Z",
        ];
        let json_dir = cli_main_args_fixture_dir("cli_validate_review_evidence_identity_json");
        let _ = std::fs::remove_dir_all(&json_dir);
        std::fs::create_dir_all(&json_dir)
            .expect("App.Cli identity JSON fixture directory の作成に失敗");
        std::fs::write(json_dir.join("input.ls"), source)
            .expect("App.Cli identity JSON fixture の書き込みに失敗");
        let json_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &wasm,
            Some(&json_dir),
            &identity_args,
            "",
        )
        .expect("App.Cli identity JSON の実行に失敗");
        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(json_dir.join("intent-graph.json"))
                .expect("App.Cli identity manifest は出力されるべき"),
        )
        .expect("App.Cli identity manifest は valid JSON であるべき");
        let _ = std::fs::remove_dir_all(&json_dir);

        assert_eq!(
            json_output.exit_code, 2,
            "identity を付けた未検証 review は unknown のままにするべき: stdout={:?}",
            json_output.stdout
        );
        let report: Value = serde_json::from_str(json_output.stdout.trim())
            .expect("App.Cli explicit identity report は valid JSON であるべき");
        let expected_identity = serde_json::json!({
            "subject_digest": "sha256:graph",
            "source_commit": "commit-1",
            "artifact_digest": "sha256:artifact",
            "trust_store_digest": "sha256:trust",
            "lifecycle_digest": "sha256:lifecycle",
            "now": "2026-08-15T00:00:00Z"
        });
        assert_eq!(report["review_evidence_identity"], expected_identity);
        assert_eq!(manifest["review_evidence_identity"], expected_identity);

        let text_dir = cli_main_args_fixture_dir("cli_validate_review_evidence_identity_text");
        let _ = std::fs::remove_dir_all(&text_dir);
        std::fs::create_dir_all(&text_dir)
            .expect("App.Cli identity text fixture directory の作成に失敗");
        std::fs::write(text_dir.join("input.ls"), source)
            .expect("App.Cli identity text fixture の書き込みに失敗");
        let text_args = [
            "validate",
            "--source",
            "input.ls",
            "--format",
            "text",
            "--review-subject-digest",
            "sha256:graph",
            "--review-source-commit",
            "commit-1",
            "--review-artifact-digest",
            "sha256:artifact",
            "--review-trust-store-digest",
            "sha256:trust",
            "--review-lifecycle-digest",
            "sha256:lifecycle",
            "--review-now",
            "2026-08-15T00:00:00Z",
        ];
        let text_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_args_and_stdin_capture(
            &wasm,
            Some(&text_dir),
            &text_args,
            "",
        )
        .expect("App.Cli identity text の実行に失敗");
        let _ = std::fs::remove_dir_all(&text_dir);
        assert_eq!(text_output.exit_code, 2);
        assert_eq!(
            text_output.stdout.trim_end(),
            "status: unknown\n\
open-questions: 0\n\
independent-reviews: 0\n\
contradicting-observations: 0\n\
stale-reviews: 0\n\
stale-evidence: 0\n\
review-evidence-identity: subject=sha256:graph source=commit-1 artifact=sha256:artifact trust-store=sha256:trust lifecycle=sha256:lifecycle now=2026-08-15T00:00:00Z",
            "App.Cli text identity report は EmbeddedCli と同じ deterministic projection であるべき"
        );
    });
}

/// EC-M2-03: EmbeddedCli の invalidated review/evidence は stale facts と unknown を返す。
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_reports_stale_review_and_evidence() {
    let source = r#"
(defn stale-review []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review-001"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "pass"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-27T00:00:00Z"
    :independence "independent-review"
  :review "review:checkout/reviewer-001" "sha256:review-provenance-001" "redacted"
  :evaluates "review:checkout/reviewer-001" "evidence:checkout/review-001"
  :invalidates "change:checkout/api-v2" "review:checkout/reviewer-001"
  true)
"#;
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "embedded_validate_stale_review",
            source,
            &["validate", "--source", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の stale validation は unknown exit 2 を返すべき: stdout={:?}",
        output.stdout
    );
    let report: Value = serde_json::from_str(output.stdout.trim())
        .expect("EmbeddedCli stale report は valid JSON であるべき");
    assert_eq!(report["status"], "unknown");
    assert_eq!(report["stale_reviews"], 1);
    assert_eq!(report["stale_evidence"], 1);
}

/// EC-M3-03: EmbeddedCli の contradiction は fail report と exit 1 になること
#[test]
fn test_e2e_selfhost_embedded_cli_validate_source_reports_fail() {
    let source = r#"
(defn verify []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/rejects" "Shipped orders are rejected"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/rejects"
  :tested-by "claim:checkout/rejects" "contract:checkout/review"
  :evidence "evidence:checkout/review"
    :subject "claim:checkout/rejects"
    :method "review"
    :outcome "contradicted"
    :runner "reviewer"
    :target "aarch64-apple-darwin"
    :source-commit "deadbeef"
    :artifact-digest "sha256:abc"
    :cases 1
    :seed 42
    :generator "checkout-review"
    :producer "lsharp-test"
    :tool-version "0.2"
    :timestamp "2026-07-25T00:00:00Z"
    :independence "independent-review"
  :contradicts "evidence:checkout/review" "claim:checkout/rejects"
  true)
"#;
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "embedded_validate_fail",
            source,
            &["validate", "--source", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 1,
        "contradicting evidence は fail exit 1 であるべき: stdout={:?}",
        output.stdout
    );
    let report: Value = serde_json::from_str(output.stdout.trim())
        .expect("EmbeddedCli fail report は valid JSON であるべき");
    assert_eq!(report["status"], "fail");
    assert_eq!(report["trace_gaps"].as_array().unwrap().len(), 0);
    assert_eq!(report["open_questions"], 0);
    assert_eq!(report["independent_reviews"], 1);
    assert_eq!(report["contradicting_observations"], 1);
}

/// TEST-CLI-02-AP2: actual Cli main は自己再帰 top-level defn を typecheck できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_check_recursive_fib() {
    let lines = run_cli_main_with_input_file(
        "check_recursive_fib",
        "(defn fib [n] (if (<= n 1) n (+ (fib (- n 1)) (fib (- n 2))))) (defn main [] (fib 10))",
        &["check", "input.ls"],
    );

    assert_output_lines(
        &lines,
        &["Fn", "diagnostics:0"],
        "Cli main check argv は自己再帰 fib を false type error にしてはならない",
    );
}

/// TEST-CLI-02-AP3: actual Cli main は相互再帰 top-level defn を typecheck できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_check_mutual_recursion_even_odd() {
    let lines = run_cli_main_with_input_file(
        "check_mutual_recursion",
        "(defn even [n] (if (<= n 0) true (odd (- n 1)))) (defn odd [n] (if (<= n 0) false (even (- n 1)))) (defn main [] (even 10))",
        &["check", "input.ls"],
    );

    assert_output_lines(
        &lines,
        &["Fn", "diagnostics:0"],
        "Cli main check argv は相互再帰 even/odd を false type error にしてはならない",
    );
}

/// TEST-CLI-02-AQ: actual Cli main は argv 経由で test file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_test_file() {
    let lines = run_cli_main_with_input_file(
        "test",
        "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))",
        &["test", "input.ls"],
    );

    assert_output_lines(
        &lines,
        &["examples:2", "invariants:1", "failures:0"],
        "Cli main test argv は metadata summary を返すべき",
    );
}

/// TEST-CLI-02-AR: actual Cli main は argv 経由で review file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_review_file() {
    let lines = run_cli_main_with_input_file(
        "review",
        "(defn main [] (let [x 42] 0))",
        &["review", "input.ls"],
    );

    assert_output_lines(
        &lines,
        &[
            "1",
            "unused-let",
            "diagnostics:1,first-body:let binding x is not used",
            "warning",
            "L0001@1:1",
        ],
        "Cli main review argv は deterministic review summary を返すべき",
    );
}

/// TEST-CLI-02-AR2: actual Cli main は argv 経由で review --json を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_review_json_file() {
    let lines = run_cli_main_with_input_file(
        "review_json",
        "(defn first [] (let [unused 42] 0)) (defn second [] (do))",
        &["review", "input.ls", "--json"],
    );

    let actual: Value =
        serde_json::from_str(&lines[0]).expect("review --json output は valid JSON");
    assert_eq!(
        actual,
        doctools_json_snapshot("review-schema-object.json"),
        "Cli main review --json argv は review schema snapshot と一致するべき",
    );
}

/// TEST-CLI-02-AR2b: actual Cli main は argv 経由で review --format json を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_review_format_json_file() {
    let lines = run_cli_main_with_input_file(
        "review_format_json",
        "(defn first [] (let [unused 42] 0)) (defn second [] (do))",
        &["review", "input.ls", "--format", "json"],
    );

    let actual: Value =
        serde_json::from_str(&lines[0]).expect("review --format json output は valid JSON");
    assert_eq!(
        actual,
        doctools_json_snapshot("review-schema-object.json"),
        "Cli main review --format json argv は review schema snapshot と一致するべき",
    );
}

/// TEST-CLI-02-AR3: actual Cli main は invalid な review --format value を拒否すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_review_invalid_format_fails() {
    let output = run_cli_main_with_input_file_capture(
        "review_invalid_format",
        "(defn first [] (let [unused 42] 0))",
        &["review", "input.ls", "--format", "yaml"],
    );

    assert_eq!(
        output.exit_code, 1,
        "Cli main review --format yaml argv は exit code 1 を返すべき",
    );

    let lines = output_lines(output.stdout);
    assert_output_lines(
        &lines,
        &["error: unsupported option: yaml"],
        "Cli main review --format yaml argv は unsupported option error を返すべき",
    );
}

/// TEST-CLI-02-AS: actual Cli main は argv 経由で doc file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_file() {
    let lines = run_cli_main_with_input_file("doc", "(defn main [] 42)", &["doc", "input.ls"]);

    assert_output_lines(
        &lines,
        &["module-global", "functions:1,types:0,first-fn:main"],
        "Cli main doc argv は deterministic title/body を返すべき",
    );
}

/// TEST-CLI-02-AS2: actual Cli main は argv 経由で doc --json を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_json_file() {
    let lines = run_cli_main_with_input_file(
        "doc_json",
        "(module Demo)\n(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))\n(type Doc Int)\n(type-alias Alias Int)",
        &["doc", "input.ls", "--json"],
    );

    let actual: Value = serde_json::from_str(&lines[0]).expect("doc --json output は valid JSON");
    assert_eq!(
        actual,
        doctools_json_snapshot("doc-output-schema-object.json"),
        "Cli main doc --json argv は doc-output schema snapshot と一致するべき",
    );
}

/// TEST-CLI-02-AS2b: actual Cli main は argv 経由で doc --format json を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_format_json_file() {
    let lines = run_cli_main_with_input_file(
        "doc_format_json",
        "(module Demo)\n(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" :doc \"Add two ints\" :example [(add 1 2)] (+ x y))\n(type Doc Int)\n(type-alias Alias Int)",
        &["doc", "input.ls", "--format", "json"],
    );

    let actual: Value =
        serde_json::from_str(&lines[0]).expect("doc --format json output は valid JSON");
    assert_eq!(
        actual,
        doctools_json_snapshot("doc-output-schema-object.json"),
        "Cli main doc --format json argv は doc-output schema snapshot と一致するべき",
    );
}

/// TEST-CLI-02-AS3: actual Cli main は invalid な doc --format value を拒否すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_invalid_format_fails() {
    let output = run_cli_main_with_input_file_capture(
        "doc_invalid_format",
        "(defn main [] 42)",
        &["doc", "input.ls", "--format", "yaml"],
    );

    assert_eq!(
        output.exit_code, 1,
        "Cli main doc --format yaml argv は exit code 1 を返すべき",
    );

    let lines = output_lines(output.stdout);
    assert_output_lines(
        &lines,
        &["error: unsupported option: yaml"],
        "Cli main doc --format yaml argv は unsupported option error を返すべき",
    );
}

/// TEST-CLI-02-AT: actual Cli main は argv 経由で doc-ack file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_ack_file() {
    let lines =
        run_cli_main_with_input_file("doc_ack", "(defn main [] 42)", &["doc-ack", "input.ls"]);

    assert_output_lines(
        &lines,
        &[
            "ack:recorded",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Reviewed-By: anonymous",
        ],
        "Cli main doc-ack argv は trailer を含む payload text を返すべき",
    );
}

/// TEST-CLI-02-AU: actual Cli main は argv 経由で doc-check file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_check_file() {
    let lines =
        run_cli_main_with_input_file("doc_check", "(defn main [] 42)", &["doc-check", "input.ls"]);

    assert_output_lines(
        &lines,
        &[
            "status:ok",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: anonymous",
        ],
        "Cli main doc-check argv は trailer を含む payload text を返すべき",
    );
}

/// TEST-CLI-02-AU2: actual Cli main は argv 経由で doc-ack --trailer を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_ack_trailer_only() {
    let lines = run_cli_main_with_input_file(
        "doc_ack_trailer",
        "(defn main [] 42)",
        &["doc-ack", "input.ls", "--trailer"],
    );

    assert_output_lines(
        &lines,
        &["; Doc-Reviewed-By: anonymous"],
        "Cli main doc-ack --trailer argv は comment trailer のみを返すべき",
    );
}

/// TEST-CLI-02-AU3: actual Cli main は argv 経由で doc-check --strict を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_check_strict_file() {
    let lines = run_cli_main_with_input_file(
        "doc_check_strict",
        "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
        &["doc-check", "input.ls", "--strict"],
    );

    assert_output_lines(
        &lines,
        &[
            "status:ok",
            "module-global",
            "functions:1,types:0,first-fn:main",
            "; Doc-Review-Status: Passed",
            "; Doc-Reviewed-By: anonymous",
        ],
        "Cli main doc-check --strict argv は valid trailer comment を受理するべき",
    );
}

/// TEST-CLI-02-AU4: actual Cli main は argv 経由で invalid な doc-check --strict trailer を拒否すること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_doc_check_strict_missing_trailer() {
    let output = run_cli_main_with_input_file_capture(
        "doc_check_strict_fail",
        "(defn main [] 42)\n",
        &["doc-check", "input.ls", "--strict"],
    );

    assert_eq!(
        output.exit_code, 1,
        "Cli main doc-check --strict argv は invalid trailer で exit code 1 を返すべき",
    );

    let lines = output_lines(output.stdout);
    assert_output_lines(
        &lines,
        &["error: invalid doc trailer: expected trailing comment lines"],
        "Cli main doc-check --strict argv は trailer 欠落時にエラーを返すべき",
    );
}

/// TEST-CLI-02-AV: actual Cli main は argv 経由で install command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_install_package() {
    let lines = run_cli_main_with_args(&["install", "core"]);

    assert_output_lines(
        &lines,
        &["package:core", "status:planned"],
        "Cli main install argv は deterministic dry-run plan text を返すべき",
    );
}

/// TEST-CLI-02-AW: actual Cli main は argv 経由で repl command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_repl_summary() {
    let lines = run_cli_main_with_args(&["repl"]);

    assert_output_lines(
        &lines,
        &["type:Int", "evals:1", "input-bytes:17"],
        "Cli main repl argv は warmup session summary を返すべき",
    );
}

/// TEST-CLI-02-AX: actual Cli main は argv 経由で lsp command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_lsp_summary() {
    let lines = run_cli_main_with_args(&["lsp"]);

    assert_output_lines(
        &lines,
        &[
            "sync:full",
            "hover:true",
            "completion:true",
            "definition:true",
            "references:true",
            "rename:true",
            "formatting:true",
            "requests:1",
            "documents:0",
            "source-bytes:0",
        ],
        "Cli main lsp argv は capability + shared-state summary を返すべき",
    );
}

/// TEST-CLI-02-AY: actual Cli main は argv 経由で fmt file command を処理できること
#[test]
#[ignore]
fn test_e2e_selfhost_cli_main_with_args_fmt_file() {
    let lines = run_cli_main_with_input_file("fmt", "(defn a [] 42)", &["fmt", "input.ls"]);

    assert_output_lines(
        &lines,
        &["(defn a [] 42)"],
        "Cli main fmt argv は canonical text を返すべき",
    );
}
