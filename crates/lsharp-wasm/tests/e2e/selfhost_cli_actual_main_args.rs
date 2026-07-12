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
    let dir = cli_main_args_fixture_dir(prefix);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), source).expect("fixture input.ls の書き込みに失敗");

    let wasm = compile_only(selfhost_cli_runtime_bundle());
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
        let _ = std::fs::remove_dir_all(&dir);

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
            .unwrap_or_else(|err| panic!("{command} -o output は valid Wasm であるべき: {err}"));
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
