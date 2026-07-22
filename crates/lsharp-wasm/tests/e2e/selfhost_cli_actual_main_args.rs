use super::support::*;
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

fn assurance_text_expected_lines(runner: &str, target: &str) -> Vec<String> {
    vec![
        "schema_version: 1".to_string(),
        "implementation_conformance.status: pass".to_string(),
        "implementation_conformance.method: sampled-property".to_string(),
        "implementation_conformance.generator: legacy-deterministic-smoke".to_string(),
        "implementation_conformance.contracts: 1".to_string(),
        "implementation_conformance.cases: 5".to_string(),
        "implementation_conformance.discarded_cases: unknown".to_string(),
        "implementation_conformance.seed: 0".to_string(),
        "implementation_conformance.shrinks: []".to_string(),
        "implementation_conformance.coverage.executed: 5".to_string(),
        "implementation_conformance.coverage.failed: 0".to_string(),
        "implementation_conformance.diagnostics.count: 0".to_string(),
        "implementation_conformance.diagnostics.firstErrorCode: 0".to_string(),
        "implementation_conformance.diagnostics.firstErrorSpan.start: 0".to_string(),
        "implementation_conformance.diagnostics.firstErrorSpan.end: 0".to_string(),
        "implementation_conformance.diagnostics.message: unknown".to_string(),
        format!("implementation_conformance.runner: {runner}"),
        format!("implementation_conformance.target: {target}"),
        "implementation_conformance.provenance.producer: lsharp-selfhost".to_string(),
        "implementation_conformance.provenance.tool_version: 0.1.0".to_string(),
        "implementation_conformance.provenance.source_digest: unknown".to_string(),
        "implementation_conformance.provenance.source_commit: unknown".to_string(),
        "implementation_conformance.provenance.artifact_digest: unknown".to_string(),
        "implementation_conformance.provenance.timestamp: unknown".to_string(),
        "intent_validation.status: unknown".to_string(),
        "intent_validation.open_questions: unknown".to_string(),
        "intent_validation.independent_reviews: unknown".to_string(),
        "intent_validation.contradicting_observations: unknown".to_string(),
    ]
}

fn assert_preflight_text_report(
    output: &lsharp_wasm::wasi_runner::ExecutionOutput,
    runner: &str,
    target: &str,
) {
    assert_eq!(
        output.exit_code, 2,
        "text preflight failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout.clone());
    assert_eq!(
        lines.len(),
        28,
        "text preflight failure は deterministic assurance report だけを返すべき"
    );
    assert_eq!(lines[0], "schema_version: 1");
    assert_eq!(lines[1], "implementation_conformance.status: fail");
    assert_eq!(lines[2], "implementation_conformance.method: sampled-property");
    assert_eq!(lines[3], "implementation_conformance.generator: legacy-deterministic-smoke");
    assert_eq!(lines[4], "implementation_conformance.contracts: 1");
    assert_eq!(lines[5], "implementation_conformance.cases: 0");
    assert_eq!(lines[6], "implementation_conformance.discarded_cases: unknown");
    assert_eq!(lines[9], "implementation_conformance.coverage.executed: 0");
    assert_eq!(lines[10], "implementation_conformance.coverage.failed: 1");
    assert_eq!(lines[11], "implementation_conformance.diagnostics.count: 1");
    assert_eq!(lines[12], "implementation_conformance.diagnostics.firstErrorCode: 3002");
    assert_eq!(lines[13], "implementation_conformance.diagnostics.firstErrorSpan.start: 0");
    assert_eq!(lines[14], "implementation_conformance.diagnostics.firstErrorSpan.end: 0");
    assert_eq!(lines[15], "implementation_conformance.diagnostics.message: unknown");
    assert_eq!(lines[16], format!("implementation_conformance.runner: {runner}"));
    assert_eq!(lines[17], format!("implementation_conformance.target: {target}"));
    assert!(
        lines.iter().all(|line| !line.contains("verified")),
        "text preflight failure は overall verified を出してはならない"
    );
}

fn assert_preflight_json_report(
    output: &lsharp_wasm::wasi_runner::ExecutionOutput,
    runner: &str,
    target: &str,
) {
    assert_eq!(
        output.exit_code, 2,
        "JSON preflight failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout.clone());
    assert_eq!(
        lines.len(),
        1,
        "JSON preflight failure は report 1 行だけを返すべき"
    );
    let report: Value = serde_json::from_str(&lines[0])
        .expect("JSON preflight failure は valid JSON report を返すべき");
    let conformance = &report["implementation_conformance"];
    assert_eq!(conformance["status"], "fail");
    assert_eq!(conformance["method"], "sampled-property");
    assert_eq!(conformance["cases"], 0);
    assert_eq!(conformance["coverage"]["executed"], 0);
    assert_eq!(conformance["coverage"]["failed"], 1);
    assert_eq!(conformance["diagnostics"]["count"], 1);
    assert_eq!(conformance["diagnostics"]["firstErrorCode"], 3002);
    assert_eq!(conformance["diagnostics"]["firstErrorSpan"]["start"], 0);
    assert_eq!(conformance["diagnostics"]["firstErrorSpan"]["end"], 0);
    assert_eq!(
        conformance["diagnostics"]["message"],
        "unknown",
        "JSON preflight は text と同じ unknown message sentinel を返すべき"
    );
    assert_eq!(conformance["provenance"]["runner"], runner);
    assert_eq!(conformance["target"], target);
    assert_eq!(conformance["provenance"]["producer"], "lsharp-selfhost");
    assert_eq!(conformance["provenance"]["tool_version"], "0.1.0");
    assert_eq!(conformance["provenance"]["source_digest"], "unknown");
    assert_eq!(conformance["provenance"]["timestamp"], "unknown");
    assert_eq!(report["intent_validation"]["status"], "unknown");
    assert!(
        !lines.iter().any(|line| line.contains("verified")),
        "JSON preflight failure は overall verified を出してはならない"
    );
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
    assert_eq!(
        report["implementation_conformance"]["target"],
        "runtime-selected",
        "App.Cli JSON report は text と同じ runtime-selected target を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["runner"],
        "selfhost-cli",
        "App.Cli JSON report は入口固有の runner provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["producer"],
        "lsharp-selfhost",
        "JSON report は text と同じ producer provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["tool_version"],
        "0.1.0",
        "JSON report は text と同じ tool_version provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["source_digest"],
        "unknown",
        "未注入 source digest は unknown を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["timestamp"],
        "unknown",
        "未注入 timestamp は unknown を返すべき"
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
    assert_eq!(report["intent_validation"]["open_questions"], 0);
    assert_eq!(report["intent_validation"]["independent_reviews"], 0);
    assert_eq!(report["intent_validation"]["contradicting_observations"], 0);
}

/// EC-M1-06: actual selfhost CLI の test --format text が JSON と同じ二軸を安定した行形式で返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_test_format_text_file() {
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "test_format_text",
            "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)",
            &["test", "input.ls", "--format", "text"],
        )
    });
    assert_eq!(
        output.exit_code, 0,
        "test --format text の passing property は exit code 0 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_output_lines(
        &lines,
        &[
            "schema_version: 1",
            "implementation_conformance.status: pass",
            "implementation_conformance.method: sampled-property",
            "implementation_conformance.generator: legacy-deterministic-smoke",
            "implementation_conformance.contracts: 1",
            "implementation_conformance.cases: 5",
            "implementation_conformance.discarded_cases: unknown",
            "implementation_conformance.seed: 0",
            "implementation_conformance.shrinks: []",
            "implementation_conformance.coverage.executed: 5",
            "implementation_conformance.coverage.failed: 0",
            "implementation_conformance.diagnostics.count: 0",
            "implementation_conformance.diagnostics.firstErrorCode: 0",
            "implementation_conformance.diagnostics.firstErrorSpan.start: 0",
            "implementation_conformance.diagnostics.firstErrorSpan.end: 0",
            "implementation_conformance.diagnostics.message: unknown",
            "implementation_conformance.runner: selfhost-cli",
            "implementation_conformance.target: runtime-selected",
            "implementation_conformance.provenance.producer: lsharp-selfhost",
            "implementation_conformance.provenance.tool_version: 0.1.0",
            "implementation_conformance.provenance.source_digest: unknown",
            "implementation_conformance.provenance.source_commit: unknown",
            "implementation_conformance.provenance.artifact_digest: unknown",
            "implementation_conformance.provenance.timestamp: unknown",
            "intent_validation.status: unknown",
            "intent_validation.open_questions: unknown",
            "intent_validation.independent_reviews: unknown",
            "intent_validation.contradicting_observations: unknown",
        ],
        "Cli main test --format text は JSON と同じ二軸 assurance summary を返すべき",
    );
    assert!(
        lines.iter().all(|line| !line.contains("verified")),
        "Cli main assurance text は overall verified を出してはならない"
    );
}

/// EC-M1-06: EmbeddedCli の test --format text が runner/target を含む同じ行契約を返すこと
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_text_file() {
    let source = "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_text_property_success_embedded",
            source,
            &["test", "input.ls", "--format", "text"],
        )
    });
    assert_eq!(
        output.exit_code, 0,
        "EmbeddedCli の test --format text passing property は exit code 0 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    let expected = assurance_text_expected_lines("selfhost-embedded-wasm", "wasm32-wasip1");
    let expected_refs: Vec<&str> = expected.iter().map(String::as_str).collect();
    assert_output_lines(
        &lines,
        &expected_refs,
        "EmbeddedCli の test --format text は Cli と同じ assurance 行契約を返すべき",
    );
    assert!(
        lines.iter().all(|line| !line.contains("verified")),
        "EmbeddedCli の assurance text は overall verified を出してはならない"
    );
}

/// EC-M1-06: EmbeddedCli の passing sampled-property report が Cli と同じ形を返すこと
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_property_success() {
    let source = "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_json_property_success_embedded",
            source,
            &["test", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 0,
        "EmbeddedCli の passing property は exit code 0 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(
        lines.len(),
        1,
        "EmbeddedCli の test --format json は JSON report だけを stdout へ返すべき"
    );
    let report: Value = serde_json::from_str(&lines[0])
        .expect("EmbeddedCli の passing property report は valid JSON であるべき");
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
    assert_eq!(
        report["implementation_conformance"]["target"],
        "wasm32-wasip1",
        "EmbeddedCli JSON report は実行 target を保持するべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["runner"],
        "selfhost-embedded-wasm",
        "EmbeddedCli JSON report は入口固有の runner provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["producer"],
        "lsharp-selfhost",
        "EmbeddedCli JSON report は text と同じ producer provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["source_commit"],
        "unknown",
        "未注入 source commit は unknown を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["artifact_digest"],
        "unknown",
        "未注入 artifact digest は unknown を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["tool_version"],
        "0.1.0",
        "EmbeddedCli JSON report は text と同じ tool_version provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["source_digest"],
        "unknown",
        "未注入 source digest は unknown を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["timestamp"],
        "unknown",
        "未注入 timestamp は unknown を返すべき"
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
}

/// EC-M1-06: EmbeddedCli の canonical :case failure が structured report へ届くこと
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_case_failure() {
    let source =
        "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_json_case_failure_embedded",
            source,
            &["test", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の canonical case failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(
        lines.len(),
        1,
        "EmbeddedCli の case failure は JSON report だけを stdout へ返すべき"
    );
    let report: Value = serde_json::from_str(&lines[0])
        .expect("EmbeddedCli の canonical case failure report は valid JSON であるべき");
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "explicit-case"
    );
    assert_eq!(report["implementation_conformance"]["cases"], 2);
    assert_eq!(report["implementation_conformance"]["coverage"]["executed"], 2);
    assert_eq!(report["implementation_conformance"]["coverage"]["failed"], 1);
    assert_eq!(report["implementation_conformance"]["diagnostics"]["count"], 0);
    assert_eq!(
        report["implementation_conformance"]["target"],
        "wasm32-wasip1",
        "EmbeddedCli JSON failure report は実行 target を保持するべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["runner"],
        "selfhost-embedded-wasm",
        "EmbeddedCli JSON failure report は入口固有の runner provenance を返すべき"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["producer"],
        "lsharp-selfhost"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["tool_version"],
        "0.1.0"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["source_commit"],
        "unknown"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["artifact_digest"],
        "unknown"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["source_digest"],
        "unknown"
    );
    assert_eq!(
        report["implementation_conformance"]["provenance"]["timestamp"],
        "unknown"
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
}

/// EC-M1-06: EmbeddedCli の text runtime failure が diagnostic failure と分離されること
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_text_case_failure() {
    let source =
        "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_text_case_failure_embedded",
            source,
            &["test", "input.ls", "--format", "text"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の text case failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(
        lines.len(),
        28,
        "EmbeddedCli の text failure は deterministic assurance report だけを返すべき"
    );
    assert_eq!(lines[0], "schema_version: 1");
    assert_eq!(lines[1], "implementation_conformance.status: fail");
    assert_eq!(lines[2], "implementation_conformance.method: explicit-case");
    assert_eq!(lines[3], "implementation_conformance.generator: direct-evaluation");
    assert_eq!(lines[4], "implementation_conformance.contracts: 2");
    assert_eq!(lines[5], "implementation_conformance.cases: 2");
    assert_eq!(lines[9], "implementation_conformance.coverage.executed: 2");
    assert_eq!(lines[10], "implementation_conformance.coverage.failed: 1");
    assert_eq!(lines[11], "implementation_conformance.diagnostics.count: 0");
    assert_eq!(lines[12], "implementation_conformance.diagnostics.firstErrorCode: 0");
    assert_eq!(lines[16], "implementation_conformance.runner: selfhost-embedded-wasm");
    assert_eq!(lines[17], "implementation_conformance.target: wasm32-wasip1");
    assert!(
        lines.iter().all(|line| !line.contains("verified")),
        "EmbeddedCli の text runtime failure は overall verified を出してはならない"
    );
}

/// EC-M1-06: App.Cli の text runtime failure が diagnostic failure と分離されること
#[test]
fn test_e2e_selfhost_cli_main_with_args_test_format_text_case_failure() {
    let source =
        "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))";
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "test_format_text_case_failure_cli",
            source,
            &["test", "input.ls", "--format", "text"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "Cli の text case failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(
        lines.len(),
        28,
        "Cli の text failure は deterministic assurance report だけを返すべき"
    );
    assert_eq!(lines[0], "schema_version: 1");
    assert_eq!(lines[1], "implementation_conformance.status: fail");
    assert_eq!(lines[2], "implementation_conformance.method: explicit-case");
    assert_eq!(lines[3], "implementation_conformance.generator: direct-evaluation");
    assert_eq!(lines[4], "implementation_conformance.contracts: 2");
    assert_eq!(lines[5], "implementation_conformance.cases: 2");
    assert_eq!(lines[9], "implementation_conformance.coverage.executed: 2");
    assert_eq!(lines[10], "implementation_conformance.coverage.failed: 1");
    assert_eq!(lines[11], "implementation_conformance.diagnostics.count: 0");
    assert_eq!(lines[12], "implementation_conformance.diagnostics.firstErrorCode: 0");
    assert_eq!(lines[16], "implementation_conformance.runner: selfhost-cli");
    assert_eq!(lines[17], "implementation_conformance.target: runtime-selected");
    assert!(
        lines.iter().all(|line| !line.contains("verified")),
        "Cli の text runtime failure は overall verified を出してはならない"
    );
}

/// EC-M1-06: Cli と EmbeddedCli の text preflight failure が JSON と同じ境界を返すこと
#[test]
fn test_e2e_selfhost_text_assurance_preflight_failure() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 3 :seed 42 :postcondition (= result value))] x)";
    let (cli_output, embedded_output) = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        (
            run_cli_main_with_input_file_capture(
                "test_format_text_preflight_cli",
                source,
                &["test", "input.ls", "--format", "text"],
            ),
            run_main_with_input_file_capture(
                selfhost_embedded_cli_runtime_bundle(),
                "test_format_text_preflight_embedded",
                source,
                &["test", "input.ls", "--format", "text"],
            ),
        )
    });

    assert_preflight_text_report(&cli_output, "selfhost-cli", "runtime-selected");
    assert_preflight_text_report(
        &embedded_output,
        "selfhost-embedded-wasm",
        "wasm32-wasip1",
    );
}

/// EC-M1-06: JSON と text の preflight failure が同じ unknown message sentinel を返すこと
#[test]
fn test_e2e_selfhost_json_assurance_preflight_failure_matches_text_boundary() {
    let source = "(defn identity [x] :property [(for-all [value Int] :cases 3 :seed 42 :postcondition (= result value))] x)";
    let (cli_output, embedded_output) = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        (
            run_cli_main_with_input_file_capture(
                "test_format_json_preflight_cli",
                source,
                &["test", "input.ls", "--format", "json"],
            ),
            run_main_with_input_file_capture(
                selfhost_embedded_cli_runtime_bundle(),
                "test_format_json_preflight_embedded",
                source,
                &["test", "input.ls", "--format", "json"],
            ),
        )
    });

    assert_preflight_json_report(&cli_output, "selfhost-cli", "runtime-selected");
    assert_preflight_json_report(
        &embedded_output,
        "selfhost-embedded-wasm",
        "wasm32-wasip1",
    );
}

/// EC-M1-06: canonical :case と sampled :property を混在させても executed を payload 値で数えないこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_test_format_json_mixed_case_property_success() {
    let source = "(defn identity [x] :case [(expect (identity 1) 1)] :property [(for-all [sample String] :cases 2 :postcondition (string-eq result sample))] x)";
    let oracle = run_metadata_tests(source);
    assert_eq!(
        oracle.len(),
        2,
        "Rust oracle は canonical case と property の論理テストを生成するべき: {oracle:?}"
    );
    assert!(
        oracle.iter().all(|result| result.passed),
        "Rust oracle の混在 suite は全件成功するべき: {oracle:?}"
    );
    let (cli_output, embedded_output) = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        (
            run_cli_main_with_input_file_capture(
                "test_format_json_mixed_case_property_cli",
                source,
                &["test", "input.ls", "--format", "json"],
            ),
            run_main_with_input_file_capture(
                selfhost_embedded_cli_runtime_bundle(),
                "test_format_json_mixed_case_property_embedded",
                source,
                &["test", "input.ls", "--format", "json"],
            ),
        )
    });

    assert_eq!(cli_output.exit_code, 0, "Cli の混在 suite は成功終了するべき");
    assert_eq!(
        embedded_output.exit_code, 0,
        "EmbeddedCli の混在 suite は成功終了するべき"
    );
    let cli_lines = output_lines(cli_output.stdout);
    let embedded_lines = output_lines(embedded_output.stdout);
    assert_eq!(cli_lines.len(), 1, "Cli は JSON report 1 行だけを返すべき");
    assert_eq!(
        embedded_lines.len(),
        1,
        "EmbeddedCli は JSON report 1 行だけを返すべき"
    );
    let cli_report: Value =
        serde_json::from_str(&cli_lines[0]).expect("Cli の混在 report は valid JSON");
    let embedded_report: Value = serde_json::from_str(&embedded_lines[0])
        .expect("EmbeddedCli の混在 report は valid JSON");

    for (name, report) in [("Cli", &cli_report), ("EmbeddedCli", &embedded_report)] {
        assert_eq!(
            report["implementation_conformance"]["status"],
            "pass",
            "{name} の混在 suite は pass report を返すべき"
        );
        assert_eq!(
            report["implementation_conformance"]["method"],
            "sampled-property",
            "{name} は property を含む混在 suite を sampled-property として報告するべき"
        );
        assert_eq!(
            report["implementation_conformance"]["cases"],
            3,
            "{name} の cases は canonical case 1 件 + property 2 件であるべき"
        );
        assert_eq!(
            report["implementation_conformance"]["coverage"]["executed"],
            3,
            "{name} の executed は case actual 値の合計ではなく実行件数であるべき"
        );
        assert_eq!(
            report["implementation_conformance"]["coverage"]["failed"],
            0,
            "{name} の混在 suite に失敗はないべき"
        );
    }
    assert_eq!(
        cli_report, embedded_report,
        "Cli と EmbeddedCli は混在 suite でも同じ structured report を返すべき"
    );
}

/// EC-M1-06: mixed suite の case failure が property samples と分離された failure coverage になること
#[test]
fn test_e2e_selfhost_embedded_cli_main_with_args_test_format_json_mixed_case_property_failure() {
    let source = "(defn identity [x] :case [(expect (identity 1) 2)] :property [(for-all [sample String] :cases 2 :postcondition (string-eq result sample))] x)";
    let oracle = run_metadata_tests(source);
    assert_eq!(
        oracle.len(),
        2,
        "Rust oracle は mixed suite の logical case/property を生成するべき: {oracle:?}"
    );
    assert!(!oracle[0].passed, "Rust oracle の canonical case は失敗するべき");
    assert!(oracle[1].passed, "Rust oracle の property は成功するべき");

    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_main_with_input_file_capture(
            selfhost_embedded_cli_runtime_bundle(),
            "test_format_json_mixed_case_property_failure_embedded",
            source,
            &["test", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "EmbeddedCli の mixed suite failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(lines.len(), 1, "mixed failure は JSON report 1 行だけを返すべき");
    let report: Value =
        serde_json::from_str(&lines[0]).expect("mixed failure report は valid JSON");
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "sampled-property"
    );
    assert_eq!(report["implementation_conformance"]["cases"], 3);
    assert_eq!(
        report["implementation_conformance"]["coverage"]["executed"],
        3
    );
    assert_eq!(
        report["implementation_conformance"]["coverage"]["failed"],
        1
    );
    assert_eq!(report["implementation_conformance"]["diagnostics"]["count"], 0);
    assert_eq!(report["intent_validation"]["status"], "unknown");
}

/// EC-M1-06: App.Cli の mixed failure report が EmbeddedCli と同じ coverage boundary を返すこと
#[test]
fn test_e2e_selfhost_cli_main_with_args_test_format_json_mixed_case_property_failure() {
    let source = "(defn identity [x] :case [(expect (identity 1) 2)] :property [(for-all [sample String] :cases 2 :postcondition (string-eq result sample))] x)";
    let oracle = run_metadata_tests(source);
    assert_eq!(
        oracle.len(),
        2,
        "Rust oracle は mixed suite の logical case/property を生成するべき: {oracle:?}"
    );
    assert!(!oracle[0].passed, "Rust oracle の canonical case は失敗するべき");
    assert!(oracle[1].passed, "Rust oracle の property は成功するべき");

    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, || {
        run_cli_main_with_input_file_capture(
            "test_format_json_mixed_case_property_failure_cli",
            source,
            &["test", "input.ls", "--format", "json"],
        )
    });

    assert_eq!(
        output.exit_code, 2,
        "Cli の mixed suite failure は exit code 2 で終了するべき"
    );
    let lines = output_lines(output.stdout);
    assert_eq!(lines.len(), 1, "Cli mixed failure は JSON report 1 行だけを返すべき");
    let report: Value = serde_json::from_str(&lines[0]).expect("Cli mixed failure report は valid JSON");
    assert_eq!(report["implementation_conformance"]["status"], "fail");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "sampled-property"
    );
    assert_eq!(report["implementation_conformance"]["cases"], 3);
    assert_eq!(
        report["implementation_conformance"]["coverage"]["executed"],
        3
    );
    assert_eq!(
        report["implementation_conformance"]["coverage"]["failed"],
        1
    );
    assert_eq!(report["implementation_conformance"]["diagnostics"]["count"], 0);
    assert_eq!(report["intent_validation"]["status"], "unknown");
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
