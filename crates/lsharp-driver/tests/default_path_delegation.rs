#![cfg(unix)]

use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("lsharp_{name}_{}_{}", std::process::id(), nonce));
    fs::create_dir_all(&dir).expect("temp dir creation failed");
    dir
}

fn write_executable_script(path: &Path, body: &str) {
    fs::write(path, body).expect("script write failed");
    let mut perms = fs::metadata(path)
        .expect("script metadata failed")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("script chmod failed");
}

fn write_source_file(path: &Path, body: &str) {
    fs::write(path, body).expect("source write failed");
}

fn write_component_file(path: &Path, wat_source: &str) {
    let bytes = wat::parse_str(wat_source).expect("component wat parse failed");
    fs::write(path, bytes).expect("component write failed");
}

fn compile_preview1_source(source: &str) -> Vec<u8> {
    let program = lsharp_syntax::parse(source).expect("parse failed");
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).expect("infer failed");
    let mut lower = Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .expect("lower failed");
    lsharp_wasm::wasi::emit_wasm_wasi(&module).expect("wasi emit failed")
}

fn compile_preview1_entry(entry_file: &Path) -> Vec<u8> {
    let module = lsharp_ir::compile_multi_file(entry_file).expect("multi-file compile failed");
    lsharp_wasm::wasi::emit_wasm_wasi(&module).expect("wasi emit failed")
}

fn compile_component_entry(entry_file: &Path) -> Vec<u8> {
    let module = lsharp_ir::compile_multi_file(entry_file).expect("multi-file compile failed");
    lsharp_wasm::wasi::emit_wasm_wasi_p2(&module).expect("component emit failed")
}

fn build_driver_with_embedded_component(
    project_root: &Path,
    component_path: &Path,
    target_dir: &Path,
) -> PathBuf {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .arg("build")
        .arg("-q")
        .arg("-p")
        .arg("lsharp-driver")
        .arg("--bin")
        .arg("lsharp")
        .arg("--target-dir")
        .arg(target_dir)
        .env("LSHARP_EMBED_COMPONENT_PATH", component_path)
        .current_dir(project_root)
        .output()
        .expect("embedded component build failed");
    assert!(
        output.status.success(),
        "embedded component build should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    target_dir.join("debug").join("lsharp")
}

fn copy_executable_binary(source: &Path, dest: &Path) {
    fs::copy(source, dest).expect("binary copy failed");
    let mut perms = fs::metadata(dest)
        .expect("binary metadata failed")
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(dest, perms).expect("binary chmod failed");
}

#[test]
fn test_driver_delegates_to_lsharp_path_executable() {
    let temp_dir = unique_temp_dir("default_path_exec");
    let script_path = temp_dir.join("delegate.sh");
    write_executable_script(
        &script_path,
        r#"#!/usr/bin/env bash
set -euo pipefail
echo "delegated-exec:$*"
exit 17
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("parse")
        .arg("dummy.ls")
        .env("LSHARP_PATH", &script_path)
        .output()
        .expect("driver execution failed");

    assert_eq!(
        output.status.code(),
        Some(17),
        "LSHARP_PATH executable の終了コードをそのまま返すべき"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("delegated-exec:parse dummy.ls"),
        "driver は LSHARP_PATH executable へ引数を委譲するべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_delegates_to_lsharp_path_directory() {
    let temp_dir = unique_temp_dir("default_path_dir");
    let script_path = temp_dir.join("lsharp");
    write_executable_script(
        &script_path,
        r#"#!/usr/bin/env bash
set -euo pipefail
echo "delegated-dir:$*"
exit 23
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("--version")
        .env("LSHARP_PATH", &temp_dir)
        .output()
        .expect("driver execution failed");

    assert_eq!(
        output.status.code(),
        Some(23),
        "LSHARP_PATH directory 配下の lsharp の終了コードを返すべき"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("delegated-dir:--version"),
        "driver は LSHARP_PATH directory 配下の lsharp に委譲するべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、check は LSHARP_PATH なしでも動くべき
#[test]
fn test_check_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_check");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("check")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の check は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("diagnostics:0"),
        "embedded guest default path の check は selfhost 出力を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// embedded guest default path の check は current_dir 配下の absolute path 入力も受理するべき
#[test]
fn test_check_with_absolute_input_path_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_check_absolute");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("check")
        .arg(&source_path)
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の absolute check は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("diagnostics:0"),
        "embedded guest default path の absolute check は selfhost 出力を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、parse は LSHARP_PATH なしでも動くべき
#[test]
fn test_parse_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_parse");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("parse")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の parse は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("decls:1") && stdout.contains("diagnostics:0"),
        "embedded guest default path の parse は selfhost 出力を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// embedded fmt は large stdout を含む入力でも source roundtrip を返すべき
#[test]
fn test_fmt_without_lsharp_path_uses_embedded_component_default_path_for_large_file() {
    let temp_dir = unique_temp_dir("embedded_default_fmt");
    let source_path = temp_dir.join("input.ls");
    let source = format!(";; {}\n(defn main [] 42)\n", "x".repeat(5000));
    write_source_file(&source_path, &source);

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("fmt")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の fmt は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout == source,
        "embedded guest default path の fmt は source roundtrip を返すべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// embedded guest default path の fmt は current_dir 配下の absolute path 入力も受理するべき
#[test]
fn test_fmt_with_absolute_input_path_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_fmt_absolute");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("fmt")
        .arg(&source_path)
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の absolute fmt は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        fs::read_to_string(&source_path).expect("source read failed"),
        "embedded guest default path の absolute fmt は source roundtrip を返すべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、compile は LSHARP_PATH なしでも動くべき
#[test]
fn test_compile_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_compile");
    let source_path = temp_dir.join("input.ls");
    let output_path = temp_dir.join("input.component.wasm");
    write_source_file(&source_path, "(defn main [] (print 42))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("input.ls")
        .arg("-o")
        .arg("input.component.wasm")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の compile は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wasm-size:"),
        "embedded guest default path の compile は selfhost summary を返すべき: {stdout}"
    );
    let written = fs::read(&output_path).expect("compile output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "embedded guest default path の compile は Wasm/Component bytes を出力するべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_component(&written)
        .expect("compile output should be runnable component");
    assert_eq!(
        runtime_output, "42\n",
        "embedded guest default path の compile output は wasmtime で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// embedded guest default path の compile は current_dir 配下の absolute path 入力も受理するべき
#[test]
fn test_compile_with_absolute_input_path_without_lsharp_path_uses_embedded_component_default_path()
{
    let temp_dir = unique_temp_dir("embedded_default_compile_absolute");
    let source_path = temp_dir.join("input.ls");
    let output_path = temp_dir.join("input.component.wasm");
    write_source_file(&source_path, "(defn main [] (print 42))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg(&source_path)
        .arg("-o")
        .arg(&output_path)
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の absolute compile は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wasm-size:"),
        "embedded guest default path の absolute compile は selfhost summary を返すべき: {stdout}"
    );
    let written = fs::read(&output_path).expect("compile output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "embedded guest default path の absolute compile は Wasm/Component bytes を出力するべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、build は LSHARP_PATH なしでも動くべき
#[test]
fn test_build_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_build");
    let source_path = temp_dir.join("input.ls");
    let default_output_path = temp_dir.join("input.component.wasm");
    write_source_file(&source_path, "(defn main [] (print 42))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("build")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の build は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wasm-size:"),
        "embedded guest default path の build は selfhost summary を返すべき: {stdout}"
    );
    let written = fs::read(&default_output_path).expect("build output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "embedded guest default path の build は Wasm/Component bytes を出力するべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_component(&written)
        .expect("build output should be runnable component");
    assert_eq!(
        runtime_output, "42\n",
        "embedded guest default path の build output は wasmtime で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_compile_with_preview1_target_without_lsharp_path_writes_runnable_wasm_artifact() {
    let temp_dir = unique_temp_dir("embedded_default_compile_preview1");
    let source_path = temp_dir.join("input.ls");
    let output_path = temp_dir.join("input.wasm");
    write_source_file(&source_path, "(defn main [] (print 42))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("input.ls")
        .arg("--target")
        .arg("wasi-preview1")
        .arg("-o")
        .arg("input.wasm")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "preview1 compile は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wasm-size:"),
        "preview1 compile は selfhost summary を返すべき: {stdout}"
    );
    let written = fs::read(&output_path).expect("preview1 output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "preview1 compile は Wasm bytes を出力するべき"
    );
    let runtime_output =
        lsharp_wasm::wasi_runner::run_wasm_wasi(&written).expect("preview1 output should run");
    assert_eq!(
        runtime_output, "42\n",
        "preview1 compile output は preview1 runtime で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、test は LSHARP_PATH なしでも動くべき
#[test]
fn test_test_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_test");
    let source_path = temp_dir.join("input.ls");
    write_source_file(
        &source_path,
        r#"(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("test")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の test は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("examples:1")
            && stdout.contains("invariants:1")
            && stdout.contains("failures:0"),
        "embedded guest default path の test は selfhost summary を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// embedded guest default path の test は current_dir 配下の absolute path 入力も受理するべき
#[test]
fn test_test_with_absolute_input_path_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_test_absolute");
    let source_path = temp_dir.join("input.ls");
    write_source_file(
        &source_path,
        r#"(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("test")
        .arg(&source_path)
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の absolute test は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("examples:1")
            && stdout.contains("invariants:1")
            && stdout.contains("failures:0"),
        "embedded guest default path の absolute test は selfhost summary を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、review は LSHARP_PATH なしでも動くべき
#[test]
fn test_review_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_review");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] (let [x 42] 0))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("review")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の review は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("unused-let")
            && stdout.contains("diagnostics:1,first-body:let binding x is not used")
            && stdout.contains("warning")
            && stdout.contains("L0001@1:1"),
        "embedded guest default path の review は selfhost summary を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、review --json は LSHARP_PATH なしでも動くべき
#[test]
fn test_review_json_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_review_json");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] (let [unused 42] 0))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("review")
        .arg("input.ls")
        .arg("--json")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の review --json は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value = serde_json::from_str(&stdout)
        .expect("embedded review --json output は valid JSON であるべき");
    assert_eq!(
        actual,
        serde_json::json!({
            "source": "source-200",
            "diagnostics": [{
                "title": "unused-let",
                "severity": "warning",
                "message": "let binding unused is not used",
                "line": 1,
                "column": 1,
                "code": "L0001"
            }]
        }),
        "embedded guest default path の review --json は schema-object review JSON を返すべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、review --format json は LSHARP_PATH なしでも動くべき
#[test]
fn test_review_format_json_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_review_format_json");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] (let [unused 42] 0))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("review")
        .arg("input.ls")
        .arg("--format")
        .arg("json")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の review --format json は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value = serde_json::from_str(&stdout)
        .expect("embedded review --format json output は valid JSON であるべき");
    assert_eq!(
        actual,
        serde_json::json!({
            "source": "source-200",
            "diagnostics": [{
                "title": "unused-let",
                "severity": "warning",
                "message": "let binding unused is not used",
                "line": 1,
                "column": 1,
                "code": "L0001"
            }]
        }),
        "embedded guest default path の review --format json は schema-object review JSON を返すべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、doc-ack は LSHARP_PATH なしでも動くべき
#[test]
fn test_doc_ack_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_doc_ack");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-ack")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の doc-ack は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ack:recorded")
            && stdout.contains("module-global")
            && stdout.contains("functions:1,types:0,first-fn:main")
            && stdout.contains("Doc-Reviewed-By: anonymous"),
        "embedded guest default path の doc-ack は selfhost summary を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、doc-ack --trailer も LSHARP_PATH なしで動くべき
#[test]
fn test_embedded_default_path_doc_ack_trailer_only() {
    let temp_dir = unique_temp_dir("embedded_default_doc_ack_trailer");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-ack")
        .arg("input.ls")
        .arg("--trailer")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の doc-ack --trailer は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "; Doc-Reviewed-By: anonymous",
        "embedded guest default path の doc-ack --trailer は trailer のみを返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、doc-check は LSHARP_PATH なしでも動くべき
#[test]
fn test_doc_check_without_lsharp_path_uses_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("embedded_default_doc_check");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-check")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の doc-check は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("status:ok")
            && stdout.contains("module-global")
            && stdout.contains("functions:1,types:0,first-fn:main")
            && stdout.contains("Doc-Review-Status: Passed")
            && stdout.contains("Doc-Reviewed-By: anonymous"),
        "embedded guest default path の doc-check は selfhost summary を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、doc-check --strict は valid trailer を受理するべき
#[test]
fn test_embedded_default_path_doc_check_strict_valid_trailer() {
    let temp_dir = unique_temp_dir("embedded_default_doc_check_strict");
    let source_path = temp_dir.join("input.ls");
    write_source_file(
        &source_path,
        "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-check")
        .arg("input.ls")
        .arg("--strict")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "embedded guest default path の doc-check --strict は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("status:ok")
            && stdout.contains("module-global")
            && stdout.contains("functions:1,types:0,first-fn:main")
            && stdout.contains("Doc-Review-Status: Passed")
            && stdout.contains("Doc-Reviewed-By: anonymous"),
        "embedded guest default path の doc-check --strict は trailer を保持した summary を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// build.rs が既定で埋め込んだ guest component により、doc-check --strict は invalid trailer を拒否するべき
#[test]
fn test_embedded_default_path_doc_check_strict_missing_trailer_fails() {
    let temp_dir = unique_temp_dir("embedded_default_doc_check_strict_fail");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-check")
        .arg("input.ls")
        .arg("--strict")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "embedded guest default path の doc-check --strict は invalid trailer で失敗するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("error: invalid doc trailer: expected trailing comment lines"),
        "embedded guest default path の doc-check --strict は invalid trailer message を返すべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_compile_help_without_lsharp_path_uses_builtin_clap_surface() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("--help")
        .env_remove("LSHARP_PATH")
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "compile --help は built-in clap surface で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--emit-ir"),
        "compile --help は Rust driver の help text を保持するべき: {stdout}"
    );
}

#[test]
fn test_compile_with_web_wasm_target_without_lsharp_path_uses_rust_builtin_path() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp_dir = unique_temp_dir("compile_web_wasm_builtin");
    let output_path = temp_dir.join("fib.wasm");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("examples/fib.ls")
        .arg("--target")
        .arg("web-wasm")
        .arg("-o")
        .arg(&output_path)
        .env_remove("LSHARP_PATH")
        .current_dir(&project_root)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "web-wasm target は Rust built-in path で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let wasm_bytes = fs::read(&output_path).expect("web-wasm output read failed");
    assert!(
        wasm_bytes.starts_with(b"\0asm"),
        "web-wasm target は実バイナリ Wasm を出力するべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_parse_without_lsharp_path_respects_embedded_disable_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("parse")
        .arg("dummy.ls")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "disable flag 付きの parse は built-in path に戻って失敗するべき"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag は shadow command hint を復帰させるべき: {stderr}"
    );
}

#[test]
fn test_review_without_lsharp_path_respects_embedded_disable_flag() {
    let temp_dir = unique_temp_dir("embedded_disable_review");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] (let [unused 42] 0))\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("review")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "disable flag 付きの review は host YAML fallback ではなく hint に戻るべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag は review でも delegation hint を復帰させるべき: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_review_help_without_lsharp_path_keeps_builtin_clap_surface_when_embedded_disabled() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("review")
        .arg("--help")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "review --help は embedded disable 下でも built-in clap surface で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("review") && stdout.contains("Usage:"),
        "review --help は hint ではなく clap help text を返すべき: {stdout}"
    );
}

#[test]
fn test_doc_ack_without_lsharp_path_respects_embedded_disable_flag() {
    let temp_dir = unique_temp_dir("embedded_disable_doc_ack");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-ack")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "disable flag 付きの doc-ack は host tracker fallback ではなく hint に戻るべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag は doc-ack でも delegation hint を復帰させるべき: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_doc_ack_trailer_without_lsharp_path_respects_embedded_disable_flag() {
    let temp_dir = unique_temp_dir("embedded_disable_doc_ack_trailer");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-ack")
        .arg("input.ls")
        .arg("--trailer")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "disable flag 付きの doc-ack --trailer は host tracker fallback ではなく hint に戻るべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag は doc-ack --trailer でも delegation hint を復帰させるべき: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_doc_check_without_lsharp_path_respects_embedded_disable_flag() {
    let temp_dir = unique_temp_dir("embedded_disable_doc_check");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-check")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "disable flag 付きの doc-check は host validator fallback ではなく hint に戻るべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag は doc-check でも delegation hint を復帰させるべき: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_doc_check_strict_without_lsharp_path_respects_embedded_disable_flag() {
    let temp_dir = unique_temp_dir("embedded_disable_doc_check_strict");
    let source_path = temp_dir.join("input.ls");
    write_source_file(
        &source_path,
        "(defn main [] 42)\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-check")
        .arg("input.ls")
        .arg("--strict")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "disable flag 付きの doc-check --strict は host validator fallback ではなく hint に戻るべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag は doc-check --strict でも delegation hint を復帰させるべき: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_doc_ack_help_without_lsharp_path_keeps_builtin_clap_surface_when_embedded_disabled() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-ack")
        .arg("--help")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "doc-ack --help は embedded disable 下でも built-in clap surface で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("doc-ack") && stdout.contains("Usage:"),
        "doc-ack --help は hint ではなく clap help text を返すべき: {stdout}"
    );
}

#[test]
fn test_doc_check_help_without_lsharp_path_keeps_builtin_clap_surface_when_embedded_disabled() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("doc-check")
        .arg("--help")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "doc-check --help は embedded disable 下でも built-in clap surface で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("doc-check") && stdout.contains("Usage:"),
        "doc-check --help は hint ではなく clap help text を返すべき: {stdout}"
    );
}

/// check コマンドを LSHARP_PATH 設定済みで実行すると外部 compiler へ委譲されるべき
#[test]
fn test_driver_delegates_check_command_via_lsharp_path() {
    let temp_dir = unique_temp_dir("check_delegation");
    let script_path = temp_dir.join("lsharp-check.sh");
    write_executable_script(
        &script_path,
        r#"#!/usr/bin/env bash
set -euo pipefail
echo "check-delegated:$*"
exit 0
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("check")
        .arg("selfhost/src/Types/TypeInfer.ls")
        .env("LSHARP_PATH", &script_path)
        .output()
        .expect("driver execution failed");

    assert!(
        output.status.success(),
        "LSHARP_PATH 設定時の check は外部 compiler 終了コードを返すべき"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("check-delegated:check selfhost/src/Types/TypeInfer.ls"),
        "check コマンドは LSHARP_PATH 先へ argv を委譲するべき: {stdout}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_rejects_invalid_lsharp_path() {
    let temp_dir = unique_temp_dir("default_path_invalid");
    let missing_path = temp_dir.join("missing-lsharp");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("--version")
        .env("LSHARP_PATH", &missing_path)
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "不正な LSHARP_PATH は成功してはいけない"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "不正な LSHARP_PATH を明示したエラーを返すべき: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_delegates_to_wasm_cli_artifact_via_lsharp_path() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_source = project_root.join("selfhost/src/App/SmokeCli.ls");
    let temp_dir = unique_temp_dir("default_path_wasm_cli");
    let source_path = temp_dir.join("input.ls");
    let wasm_path = temp_dir.join("selfhost-cli.wasm");

    write_source_file(&source_path, "(defn main [] 42)\n");
    fs::write(&wasm_path, compile_preview1_entry(&cli_source))
        .expect("selfhost cli wasm write failed");
    assert!(wasm_path.is_file(), "selfhost cli wasm が生成されていない");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("parse")
        .arg("input.ls")
        .env("LSHARP_PATH", &wasm_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution via wasm path failed");

    assert!(
        output.status.success(),
        "LSHARP_PATH=.wasm delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("decls:1"),
        "Wasm CLI delegation は parse 出力を返すべき: {stdout}"
    );

    let check_output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("check")
        .arg("input.ls")
        .env("LSHARP_PATH", &wasm_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver check via wasm path failed");
    assert!(
        check_output.status.success(),
        "LSHARP_PATH=.wasm check delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&check_output.stdout),
        String::from_utf8_lossy(&check_output.stderr)
    );
    let check_stdout = String::from_utf8_lossy(&check_output.stdout);
    assert!(
        check_stdout.contains("check:ok") && check_stdout.contains("diagnostics:0"),
        "Wasm CLI delegation は check smoke 出力を返すべき: {check_stdout}"
    );

    let fmt_output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("fmt")
        .arg("input.ls")
        .env("LSHARP_PATH", &wasm_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver fmt via wasm path failed");
    assert!(
        fmt_output.status.success(),
        "LSHARP_PATH=.wasm fmt delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&fmt_output.stdout),
        String::from_utf8_lossy(&fmt_output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&fmt_output.stdout),
        fs::read_to_string(&source_path).expect("source read failed"),
        "Wasm CLI delegation の fmt smoke は source roundtrip を返すべき"
    );

    let compile_output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("input.ls")
        .arg("-o")
        .arg("smoke_output.txt")
        .env("LSHARP_PATH", &wasm_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver compile via wasm path failed");
    assert!(
        compile_output.status.success(),
        "LSHARP_PATH=.wasm compile delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );
    let compile_stdout = String::from_utf8_lossy(&compile_output.stdout);
    assert!(
        compile_stdout.contains("wasm-size:"),
        "Wasm CLI delegation は compile smoke 出力を返すべき: {compile_stdout}"
    );
    assert!(
        temp_dir.join("smoke_output.txt").is_file(),
        "Wasm CLI delegation は compile smoke 出力ファイルを生成するべき"
    );
    assert!(
        fs::metadata(temp_dir.join("smoke_output.txt"))
            .expect("compile output metadata failed")
            .len()
            > 0,
        "Wasm CLI delegation の compile smoke 出力ファイルは空であってはいけない"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_delegates_to_component_artifact_via_lsharp_path() {
    let temp_dir = unique_temp_dir("default_path_component_cli");
    let component_path = temp_dir.join("delegate.component.wasm");
    write_component_file(
        &component_path,
        r#"
(component
  (core module $main
    (func (export "run"))
  )
  (core instance $main (instantiate $main))
  (type (func))
  (alias core export $main "run" (core func $run))
  (func $run (type 0) (canon lift (core func $run)))
  (export "run" (func $run))
)
"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("--version")
        .env("LSHARP_PATH", &component_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution via component path failed");

    assert!(
        output.status.success(),
        "LSHARP_PATH=.component.wasm delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "component delegation は組み込み --version ではなく guest component を実行するべき: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "component delegation は不要な stderr を出さないべき: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_component_lsharp_path_compile_writes_runnable_component_artifact() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_source = project_root.join("selfhost/src/App/EmbeddedCli.ls");
    let temp_dir = unique_temp_dir("default_path_component_compile");
    let source_path = temp_dir.join("input.ls");
    let component_path = temp_dir.join("delegate.component.wasm");
    let output_path = temp_dir.join("input.component.wasm");
    write_source_file(&source_path, "(defn main [] (print 42))\n");
    fs::write(&component_path, compile_component_entry(&cli_source))
        .expect("selfhost component write failed");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("input.ls")
        .arg("-o")
        .arg("input.component.wasm")
        .env("LSHARP_PATH", &component_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver compile via component path failed");

    assert!(
        output.status.success(),
        "LSHARP_PATH=.component.wasm compile delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wasm-size:"),
        "component delegation の compile は selfhost summary を返すべき: {stdout}"
    );
    let written = fs::read(&output_path).expect("component compile output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "LSHARP_PATH=.component.wasm compile は runnable component bytes を書くべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_component(&written)
        .expect("component compile output should run");
    assert_eq!(
        runtime_output, "42\n",
        "LSHARP_PATH=.component.wasm compile output は wasmtime で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

/// Rust driver を介さず、selfhost EmbeddedCli 自身が Preview1 artifact を生成できること
#[test]
fn test_embedded_cli_component_compile_preview1_writes_runnable_wasm_without_driver() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_source = project_root.join("selfhost/src/App/EmbeddedCli.ls");
    let temp_dir = unique_temp_dir("embedded_cli_component_preview1_compile");
    let source_path = temp_dir.join("input.ls");
    let output_path = temp_dir.join("output.wasm");
    write_source_file(
        &source_path,
        "(defn inc [n] (+ n 1))\n\
         (type Point (record (: x Int) (: y Int)))\n\
         (defn main [] (let [point (Point (inc 40) 2)] (do (print (Point.x point)) (print (Point.y point)) 0)))\n",
    );
    let component = compile_component_entry(&cli_source);

    let guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "input.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "output.wasm",
            ],
        )
        .expect("embedded CLI component execution failed");
    assert_eq!(
        guest.exit_code, 0,
        "EmbeddedCli の Preview1 compile は成功するべき: stdout={}",
        guest.stdout
    );
    assert!(
        guest.stdout.contains("wasm-size:"),
        "EmbeddedCli の Preview1 compile は artifact summary を返すべき: {}",
        guest.stdout
    );

    let written = fs::read(&output_path).expect("embedded CLI Preview1 output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "EmbeddedCli の Preview1 compile は runnable Wasm bytes を書くべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&written)
        .expect("embedded CLI Preview1 output should run");
    assert_eq!(
        runtime_output, "41\n2\n",
        "EmbeddedCli の Preview1 output は static record accessor を実行できるべき"
    );

    let root_source_path = temp_dir.join("root.ls");
    let root_output_path = temp_dir.join("root.wasm");
    write_source_file(
        &root_source_path,
        "(defn main [] (let [slot (root_push 41)] (do (root_set slot 42) (print (root_pop)) 0)))\n",
    );
    let root_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "root.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "root.wasm",
            ],
        )
        .expect("embedded CLI root runtime execution failed");
    assert_eq!(
        root_guest.exit_code, 0,
        "Preview1 root runtime compile は成功するべき: stdout={}",
        root_guest.stdout
    );
    let root_written = fs::read(&root_output_path).expect("root runtime output read failed");
    let root_runtime_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&root_written)
        .expect("root runtime output should run");
    assert_eq!(
        root_runtime_output, "42\n",
        "Preview1 root_push/root_set/root_pop は値を保持するべき"
    );

    let print_string_source_path = temp_dir.join("print-string.ls");
    let print_string_output_path = temp_dir.join("print-string.wasm");
    write_source_file(
        &print_string_source_path,
        "(defn main [] (do (print-string \"hello\") 0))\n",
    );
    let print_string_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "print-string.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "print-string.wasm",
            ],
        )
        .expect("embedded CLI print-string runtime execution failed");
    assert_eq!(
        print_string_guest.exit_code, 0,
        "Preview1 print-string compile は成功するべき: stdout={}",
        print_string_guest.stdout
    );
    let print_string_written =
        fs::read(&print_string_output_path).expect("print-string runtime output read failed");
    let print_string_runtime_output =
        lsharp_wasm::wasi_runner::run_wasm_wasi(&print_string_written)
            .expect("print-string runtime output should run");
    assert_eq!(
        print_string_runtime_output, "hello",
        "Preview1 print-string は静的文字列 data を stdout へ出力するべき"
    );

    let concat_source_path = temp_dir.join("concat.ls");
    let concat_output_path = temp_dir.join("concat.wasm");
    write_source_file(
        &concat_source_path,
        "(defn main [] (do (print-string (string-concat \"hello\" \" world\")) 0))\n",
    );
    let concat_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "concat.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "concat.wasm",
            ],
        )
        .expect("embedded CLI string-concat runtime execution failed");
    assert_eq!(
        concat_guest.exit_code, 0,
        "Preview1 string-concat compile は成功するべき: stdout={}",
        concat_guest.stdout
    );
    let concat_written = fs::read(&concat_output_path).expect("string-concat output read failed");
    let concat_runtime_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&concat_written)
        .expect("string-concat runtime output should run");
    assert_eq!(
        concat_runtime_output, "hello world",
        "Preview1 string-concat は両方の文字列を結合して出力するべき"
    );

    let substring_source_path = temp_dir.join("substring.ls");
    let substring_output_path = temp_dir.join("substring.wasm");
    write_source_file(
        &substring_source_path,
        "(defn main [] (do (print-string (substring \"hello world\" 6 11)) 0))\n",
    );
    let substring_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "substring.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "substring.wasm",
            ],
        )
        .expect("embedded CLI substring runtime execution failed");
    assert_eq!(
        substring_guest.exit_code, 0,
        "Preview1 substring compile は成功するべき: stdout={}",
        substring_guest.stdout
    );
    let substring_written = fs::read(&substring_output_path).expect("substring output read failed");
    let substring_runtime_output = lsharp_wasm::wasi_runner::run_wasm_wasi(&substring_written)
        .expect("substring runtime output should run");
    assert_eq!(
        substring_runtime_output, "world",
        "Preview1 substring は指定範囲の文字列を出力するべき"
    );

    let invalid_substring_source_path = temp_dir.join("invalid-substring.ls");
    let invalid_substring_output_path = temp_dir.join("invalid-substring.wasm");
    write_source_file(
        &invalid_substring_source_path,
        "(defn main [] (do (print-string (substring \"hello\" 4 7)) 0))\n",
    );
    let invalid_substring_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "invalid-substring.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "invalid-substring.wasm",
            ],
        )
        .expect("embedded CLI invalid substring compilation failed");
    assert_eq!(
        invalid_substring_guest.exit_code, 0,
        "Preview1 の範囲外 substring は artifact 生成までは成功するべき: stdout={}",
        invalid_substring_guest.stdout
    );
    let invalid_substring_written =
        fs::read(&invalid_substring_output_path).expect("invalid substring output read failed");
    let invalid_substring_runtime =
        lsharp_wasm::wasi_runner::run_wasm_wasi(&invalid_substring_written);
    assert!(
        invalid_substring_runtime.is_err(),
        "Preview1 の範囲外 substring は runtime trap で拒否するべき"
    );

    let unsupported_source_path = temp_dir.join("unsupported.ls");
    let unsupported_output_path = temp_dir.join("unsupported.wasm");
    write_source_file(
        &unsupported_source_path,
        "(defn main [] (print-string (read-file \"missing\")))\n",
    );
    let unsupported_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "unsupported.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "unsupported.wasm",
            ],
        )
        .expect("embedded CLI unsupported runtime execution failed");
    assert_ne!(
        unsupported_guest.exit_code, 0,
        "Preview1 unsupported runtime opcode は成功扱いにしてはいけない: stdout={}",
        unsupported_guest.stdout
    );
    assert!(
        !unsupported_output_path.exists(),
        "Preview1 unsupported runtime opcode は不完全な Wasm artifact を書いてはいけない"
    );

    let large_source_path = temp_dir.join("large.ls");
    let large_output_path = temp_dir.join("large.wasm");
    let large_literal = "x".repeat(1100);
    write_source_file(
        &large_source_path,
        &format!("(defn main [] (print \"{}\"))\n", large_literal),
    );
    let large_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "large.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "large.wasm",
            ],
        )
        .expect("embedded CLI large layout execution failed");
    assert_ne!(
        large_guest.exit_code, 0,
        "Preview1 data が heap 領域へ到達する source は成功扱いにしてはいけない: stdout={}",
        large_guest.stdout
    );
    assert!(
        !large_output_path.exists(),
        "Preview1 data/heap layout violation は不完全な Wasm artifact を書いてはいけない"
    );

    let argv_source_path = temp_dir.join("argv-count.ls");
    let argv_output_path = temp_dir.join("argv-count.wasm");
    write_source_file(
        &argv_source_path,
        "(defn main [] (print (command-line-args)))\n",
    );
    let argv_guest =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_args_and_stdin_capture(
            &component,
            Some(&temp_dir),
            &[
                "compile",
                "argv-count.ls",
                "--target",
                "wasi-preview1",
                "-o",
                "argv-count.wasm",
            ],
            "",
        )
        .expect("embedded CLI argv runtime execution failed");
    assert_eq!(
        argv_guest.exit_code, 0,
        "Preview1 command-line-args compile は成功するべき: stdout={}",
        argv_guest.stdout
    );
    let argv_written = fs::read(&argv_output_path).expect("argv-count output read failed");
    let argv_runtime_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &argv_written,
        None,
        &["alpha", "beta"],
    )
    .expect("argv-count Preview1 output should run");
    assert_eq!(
        argv_runtime_output, "2\n",
        "Preview1 command-line-args は WASI argv の個数を返すべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_component_compile_absolute_input_uses_host_artifact_fallback() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_source = project_root.join("selfhost/src/App/EmbeddedCli.ls");
    let temp_dir = unique_temp_dir("default_path_component_compile_absolute_host_fallback");
    let run_dir = temp_dir.join("runner");
    let source_dir = temp_dir.join("source");
    fs::create_dir_all(&run_dir).expect("runner dir creation failed");
    fs::create_dir_all(&source_dir).expect("source dir creation failed");
    let source_path = source_dir.join("input.ls");
    let component_path = temp_dir.join("delegate.component.wasm");
    let output_path = source_dir.join("input.component.wasm");
    write_source_file(&source_path, "(defn main [] (print (+ (* 6 7) 0)))\n");
    fs::write(&component_path, compile_component_entry(&cli_source))
        .expect("selfhost component write failed");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg(&source_path)
        .arg("--target")
        .arg("wasi-component")
        .arg("-o")
        .arg(&output_path)
        .env("LSHARP_PATH", &component_path)
        .current_dir(&run_dir)
        .output()
        .expect("driver compile via component path failed");

    assert!(
        output.status.success(),
        "guest が cwd 外の絶対入力を読めなくても host artifact fallback で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let written = fs::read(&output_path).expect("fallback component output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "fallback compile は runnable component bytes を書くべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_component(&written)
        .expect("fallback component output should run");
    assert_eq!(
        runtime_output, "42\n",
        "fallback component output は host compile の意味で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_component_compile_guest_trap_uses_host_artifact_fallback() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_source = project_root.join("selfhost/src/App/EmbeddedCli.ls");
    let temp_dir = unique_temp_dir("default_path_component_compile_trap_host_fallback");
    let source_path = temp_dir.join("input.ls");
    let component_path = temp_dir.join("delegate.component.wasm");
    let output_path = temp_dir.join("input.component.wasm");
    write_source_file(
        &source_path,
        "(type (Maybe a) (Just a) Nothing)\n\
         (trait (Functor f) (defn fmap [func fa] : (f b)))\n\
         (defn identity [x] x)\n\
         (defn main [] (print (identity 42)))\n",
    );
    fs::write(&component_path, compile_component_entry(&cli_source))
        .expect("selfhost component write failed");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg("input.ls")
        .arg("--target")
        .arg("wasi-component")
        .arg("-o")
        .arg("input.component.wasm")
        .env("LSHARP_PATH", &component_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver compile via component path failed");

    assert!(
        output.status.success(),
        "guest が trap しても host artifact fallback で成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let written = fs::read(&output_path).expect("fallback component output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "fallback compile は runnable component bytes を書くべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_component(&written)
        .expect("fallback component output should run");
    assert_eq!(
        runtime_output, "42\n",
        "fallback component output は host compile の意味で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_component_lsharp_path_build_writes_runnable_component_artifact() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cli_source = project_root.join("selfhost/src/App/EmbeddedCli.ls");
    let temp_dir = unique_temp_dir("default_path_component_build");
    let source_path = temp_dir.join("input.ls");
    let component_path = temp_dir.join("delegate.component.wasm");
    let output_path = temp_dir.join("input.component.wasm");
    write_source_file(&source_path, "(defn main [] (print 42))\n");
    fs::write(&component_path, compile_component_entry(&cli_source))
        .expect("selfhost component write failed");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("build")
        .arg("input.ls")
        .env("LSHARP_PATH", &component_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver build via component path failed");

    assert!(
        output.status.success(),
        "LSHARP_PATH=.component.wasm build delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("wasm-size:"),
        "component delegation の build は selfhost summary を返すべき: {stdout}"
    );
    let written = fs::read(&output_path).expect("component build output read failed");
    assert!(
        written.starts_with(b"\0asm"),
        "LSHARP_PATH=.component.wasm build は runnable component bytes を書くべき"
    );
    let runtime_output = lsharp_wasm::wasi_runner::run_wasm_component(&written)
        .expect("component build output should run");
    assert_eq!(
        runtime_output, "42\n",
        "LSHARP_PATH=.component.wasm build output は wasmtime で実行できるべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_prefers_adjacent_component_sidecar_over_embedded_component_default_path() {
    let temp_dir = unique_temp_dir("adjacent_component_sidecar");
    let source_path = temp_dir.join("input.ls");
    let launcher_path = temp_dir.join("lsharp");
    let sidecar_path = temp_dir.join("lsharp.component.wasm");
    write_source_file(&source_path, "(defn main [] 42)\n");
    copy_executable_binary(Path::new(env!("CARGO_BIN_EXE_lsharp")), &launcher_path);
    write_component_file(
        &sidecar_path,
        r#"
(component
  (core module $main
    (func (export "run"))
  )
  (core instance $main (instantiate $main))
  (type (func))
  (alias core export $main "run" (core func $run))
  (func $run (type 0) (canon lift (core func $run)))
  (export "run" (func $run))
)
"#,
    );

    let output = Command::new(&launcher_path)
        .arg("parse")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("adjacent sidecar driver execution failed");

    assert!(
        output.status.success(),
        "adjacent sidecar default path は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "adjacent sidecar は埋め込み guest より優先されるべき: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "adjacent sidecar default path は不要な stderr を出さないべき: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_uses_embedded_component_when_compiled_with_component_path() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp_dir = unique_temp_dir("embedded_component_default");
    let component_path = temp_dir.join("embedded.component.wasm");
    let target_dir = temp_dir.join("target");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");
    write_component_file(
        &component_path,
        r#"
(component
  (core module $main
    (func (export "run"))
  )
  (core instance $main (instantiate $main))
  (type (func))
  (alias core export $main "run" (core func $run))
  (func $run (type 0) (canon lift (core func $run)))
  (export "run" (func $run))
)
"#,
    );

    let embedded_driver =
        build_driver_with_embedded_component(&project_root, &component_path, &target_dir);

    let output = Command::new(&embedded_driver)
        .arg("parse")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .current_dir(&temp_dir)
        .output()
        .expect("embedded driver execution failed");

    assert!(
        output.status.success(),
        "embedded component default path should succeed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "embedded component default path should run guest component instead of built-in parse hint: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stderr.is_empty(),
        "embedded component default path should not emit stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_can_disable_embedded_component_with_runtime_env() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp_dir = unique_temp_dir("embedded_component_disable");
    let component_path = temp_dir.join("embedded.component.wasm");
    let target_dir = temp_dir.join("target");
    let source_path = temp_dir.join("input.ls");
    write_source_file(&source_path, "(defn main [] 42)\n");
    write_component_file(
        &component_path,
        r#"
(component
  (core module $main
    (func (export "run"))
  )
  (core instance $main (instantiate $main))
  (type (func))
  (alias core export $main "run" (core func $run))
  (func $run (type 0) (canon lift (core func $run)))
  (export "run" (func $run))
)
"#,
    );

    let embedded_driver =
        build_driver_with_embedded_component(&project_root, &component_path, &target_dir);

    let output = Command::new(&embedded_driver)
        .arg("parse")
        .arg("input.ls")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("embedded driver execution with disable flag failed");

    assert!(
        !output.status.success(),
        "disable flag should keep built-in driver path available and skip guest delegation: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "disable flag should bypass embedded component and restore shadow command hint: {stderr}"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_delegates_to_wasm_artifact_and_preserves_exit_code() {
    let temp_dir = unique_temp_dir("default_path_wasm_exit_code");
    let wasm_path = temp_dir.join("delegate-exit.wasm");
    fs::write(
        &wasm_path,
        compile_preview1_source("(defn main [] (do (proc-exit 17) 0))"),
    )
    .expect("delegate wasm write failed");

    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("--version")
        .env("LSHARP_PATH", &wasm_path)
        .current_dir(&temp_dir)
        .output()
        .expect("driver execution via wasm exit path failed");

    assert_eq!(
        output.status.code(),
        Some(17),
        "Wasm delegation は guest exit code をそのまま返すべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "exit-code only guest は不要な stdout を出さないべき: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_driver_delegates_to_wasm_artifact_and_inherits_stdin() {
    let temp_dir = unique_temp_dir("default_path_wasm_stdin");
    let wasm_path = temp_dir.join("delegate-stdin.wasm");
    fs::write(
        &wasm_path,
        compile_preview1_source("(defn main [] (do (print-string (read-stdin)) 0))"),
    )
    .expect("delegate wasm write failed");

    let mut child = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("--version")
        .env("LSHARP_PATH", &wasm_path)
        .current_dir(&temp_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("driver stdin delegation spawn failed");
    child
        .stdin
        .take()
        .expect("child stdin should exist")
        .write_all(b"delegated-stdin")
        .expect("stdin write failed");
    let output = child
        .wait_with_output()
        .expect("driver stdin delegation wait failed");

    assert!(
        output.status.success(),
        "Wasm stdin delegation は成功するべき: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "delegated-stdin",
        "Wasm delegation は親 stdin を guest へ渡すべき"
    );

    let _ = fs::remove_dir_all(&temp_dir);
}
