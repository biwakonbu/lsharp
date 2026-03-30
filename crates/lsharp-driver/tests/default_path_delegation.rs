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

fn build_driver_with_embedded_component(project_root: &Path, component_path: &Path, target_dir: &Path) -> PathBuf {
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

/// check コマンドを LSHARP_PATH 未設定で実行すると LSHARP_PATH ヒントを含むエラーになるべき
#[test]
fn test_check_without_lsharp_path_suggests_lsharp_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("check")
        .arg("dummy.ls")
        .env_remove("LSHARP_PATH")
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "LSHARP_PATH 未設定時の check は失敗するべき"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "LSHARP_PATH 未設定時の check は LSHARP_PATH ヒントを含むエラーを出すべき: {stderr}"
    );
}

/// parse コマンドを LSHARP_PATH 未設定で実行すると LSHARP_PATH ヒントを含むエラーになるべき
#[test]
fn test_parse_without_lsharp_path_suggests_lsharp_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("parse")
        .arg("dummy.ls")
        .env_remove("LSHARP_PATH")
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "LSHARP_PATH 未設定時の parse は失敗するべき"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "LSHARP_PATH 未設定時の parse は LSHARP_PATH ヒントを含むエラーを出すべき: {stderr}"
    );
}

/// fmt コマンドを LSHARP_PATH 未設定で実行すると LSHARP_PATH ヒントを含むエラーになるべき
#[test]
fn test_fmt_without_lsharp_path_suggests_lsharp_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("fmt")
        .arg("dummy.ls")
        .env_remove("LSHARP_PATH")
        .output()
        .expect("driver execution failed");

    assert!(
        !output.status.success(),
        "LSHARP_PATH 未設定時の fmt は失敗するべき"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("LSHARP_PATH"),
        "LSHARP_PATH 未設定時の fmt は LSHARP_PATH ヒントを含むエラーを出すべき: {stderr}"
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

    let compile_output = Command::new(env!("CARGO_BIN_EXE_lsharp"))
        .arg("compile")
        .arg(&cli_source)
        .arg("-o")
        .arg(&wasm_path)
        .current_dir(&project_root)
        .output()
        .expect("selfhost cli compile failed");

    assert!(
        compile_output.status.success(),
        "selfhost App/SmokeCli.ls のコンパイルに失敗: stdout={}, stderr={}",
        String::from_utf8_lossy(&compile_output.stdout),
        String::from_utf8_lossy(&compile_output.stderr)
    );
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
fn test_driver_uses_embedded_component_when_compiled_with_component_path() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp_dir = unique_temp_dir("embedded_component_default");
    let component_path = temp_dir.join("embedded.component.wasm");
    let target_dir = temp_dir.join("target");
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
        .arg("--version")
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
        "embedded component default path should run guest component instead of built-in --version: {}",
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
        .arg("--version")
        .env_remove("LSHARP_PATH")
        .env("LSHARP_DISABLE_EMBEDDED_COMPONENT", "1")
        .current_dir(&temp_dir)
        .output()
        .expect("embedded driver execution with disable flag failed");

    assert!(
        output.status.success(),
        "disable flag should keep built-in driver path available: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("lsharp "),
        "disable flag should bypass embedded component and expose built-in --version output: {stdout}"
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
