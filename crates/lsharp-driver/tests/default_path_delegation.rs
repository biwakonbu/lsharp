#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
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
