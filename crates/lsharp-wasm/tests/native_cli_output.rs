use std::{path::PathBuf, process::Command};

use serde_json::Value;

fn assert_numeric_wasm_size(command: &str, stdout: &[u8]) {
    let stdout = std::str::from_utf8(stdout)
        .unwrap_or_else(|_| panic!("native {command} stdout は UTF-8 であるべき"));
    let wasm_size = stdout
        .lines()
        .find_map(|line| line.strip_prefix("wasm-size:"))
        .unwrap_or_else(|| {
            panic!("native {command} -o stdout は wasm-size:<n> を含むべき: {stdout:?}")
        });
    assert!(
        !wasm_size.is_empty(),
        "native {command} -o stdout の wasm-size は空であってはならない: {stdout:?}"
    );
    let wasm_size: u64 = wasm_size.parse().unwrap_or_else(|_| {
        panic!("native {command} -o stdout の wasm-size は数値であるべき: {stdout:?}")
    });
    assert!(
        wasm_size > 0,
        "native {command} -o stdout の wasm-size は正であるべき: {stdout:?}"
    );
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_compile_and_build_output_are_actual_wasm() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_actual_output_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    let source = "(defn main [] 42)";
    std::fs::write(dir.join("input.ls"), source).expect("fixture input.ls の書き込みに失敗");

    let result = (|| {
        let fmt_output = Command::new(&program)
            .current_dir(&dir)
            .args(["fmt", "input.ls"])
            .output()
            .expect("native App.Cli fmt の実行に失敗");
        assert!(
            fmt_output.status.success(),
            "native fmt は成功するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&fmt_output.stdout),
            String::from_utf8_lossy(&fmt_output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&fmt_output.stdout).trim(),
            source,
            "native fmt は source literal を保持するべき"
        );

        for command in ["compile", "build"] {
            let output_name = format!("{command}.wasm");
            let output = Command::new(&program)
                .current_dir(&dir)
                .args([command, "input.ls", "-o", &output_name])
                .output()
                .expect("native App.Cli の実行に失敗");

            assert!(
                output.status.success(),
                "native {command} -o は成功するべき: stdout={:?} stderr={:?}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_numeric_wasm_size(command, &output.stdout);
            let artifact = std::fs::read(dir.join(&output_name))
                .unwrap_or_else(|_| panic!("native {command} output の読み込みに失敗"));
            assert!(
                artifact.starts_with(b"\0asm\x01\0\0\0"),
                "native {command} -o は valid core Wasm を書くべき: header={:?}",
                artifact.get(..8)
            );
            wasmparser::Validator::new()
                .validate_all(&artifact)
                .unwrap_or_else(|_| panic!("native {command} output は valid Wasm であるべき"));
        }

        let component_output = Command::new(&program)
            .current_dir(&dir)
            .args([
                "compile",
                "input.ls",
                "-o",
                "component.wasm",
                "--target",
                "wasi-component",
            ])
            .output()
            .expect("native App.Cli component target の実行に失敗");
        assert!(
            !component_output.status.success(),
            "native component output は external packaging 境界として成功してはならない: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&component_output.stdout),
            String::from_utf8_lossy(&component_output.stderr)
        );
        assert!(
            !dir.join("component.wasm").exists(),
            "native component output は artifact を偽装してはならない"
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_parse_check_and_test_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_source_file_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)")
        .expect("fixture input.ls の書き込みに失敗");

    let result = (|| {
        let parse = Command::new(&program)
            .current_dir(&dir)
            .args(["parse", "input.ls"])
            .output()
            .expect("native App.Cli parse の実行に失敗");
        assert!(
            parse.status.success(),
            "native parse は成功するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&parse.stdout),
            String::from_utf8_lossy(&parse.stderr)
        );
        let parse_stdout = String::from_utf8_lossy(&parse.stdout);
        for expected in [
            "decls:1",
            "first-decl:defn",
            "first-body:int",
            "diagnostics:0",
        ] {
            assert!(
                parse_stdout.lines().any(|line| line == expected),
                "native parse は {expected:?} を出力するべき: {parse_stdout:?}"
            );
        }

        let check = Command::new(&program)
            .current_dir(&dir)
            .args(["check", "input.ls"])
            .output()
            .expect("native App.Cli check の実行に失敗");
        assert!(
            check.status.success(),
            "native check は成功するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
        let check_stdout = String::from_utf8_lossy(&check.stdout);
        for expected in ["Int", "diagnostics:0"] {
            assert!(
                check_stdout.lines().any(|line| line == expected),
                "native check は {expected:?} を出力するべき: {check_stdout:?}"
            );
        }

        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls"])
            .output()
            .expect("native App.Cli test の実行に失敗");
        assert!(
            test.status.success(),
            "native test は空 suite で成功するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&test.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["examples:0", "invariants:0", "failures:0"],
            "native test は空 suite の成功 summary を出力するべき"
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_check_parameterized_function_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_parameterized_check_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), "(defn identity [x] x)")
        .expect("fixture input.ls の書き込みに失敗");

    let result = (|| {
        let check = Command::new(&program)
            .current_dir(&dir)
            .args(["check", "input.ls"])
            .output()
            .expect("native App.Cli parameterized check の実行に失敗");
        assert!(
            check.status.success(),
            "native check は parameterized function を受理するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
        let check_stdout = String::from_utf8_lossy(&check.stdout);
        for expected in ["Fn", "diagnostics:0"] {
            assert!(
                check_stdout.lines().any(|line| line == expected),
                "native check は {expected:?} を出力するべき: {check_stdout:?}"
            );
        }
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_check_builtin_application_preserves_return_type() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_builtin_application_type_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), "(defn probe [] (not true))")
        .expect("fixture input.ls の書き込みに失敗");

    let result = (|| {
        let check = Command::new(&program)
            .current_dir(&dir)
            .args(["check", "input.ls"])
            .output()
            .expect("native App.Cli builtin application check の実行に失敗");
        assert!(
            check.status.success(),
            "native check は builtin application を受理するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
        let check_stdout = String::from_utf8_lossy(&check.stdout);
        assert!(
            check_stdout.lines().any(|line| line == "Bool"),
            "native check は builtin application の戻り値型 Bool を保持するべき: {check_stdout:?}"
        );
        assert!(
            check_stdout.lines().any(|line| line == "diagnostics:0"),
            "native check は builtin application で診断を出さないべき: {check_stdout:?}"
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_test_string_property_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_string_property_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)",
    )
    .expect("fixture input.ls の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls"])
            .output()
            .expect("native App.Cli String property test の実行に失敗");
        assert!(
            test.status.success(),
            "native test は String property を実行できるべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&test.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["examples:0", "invariants:0", "properties:1", "failures:0"],
            "native test は String property の成功 summary を出力するべき"
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_test_format_json_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_test_format_json_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [sample String] :cases 5 :postcondition (string-eq result sample))] x)",
    )
    .expect("JSON property fixture input.ls の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls", "--format", "json"])
            .output()
            .expect("native App.Cli test --format json の実行に失敗");
        assert!(
            test.status.success(),
            "native test --format json は成功するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        let stdout = String::from_utf8_lossy(&test.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "native test --format json は report 1 行を返すべき"
        );
        let report: Value =
            serde_json::from_str(lines[0]).expect("native test --format json は valid JSON");
        assert!(
            report.get("verified").is_none(),
            "native assurance report は top-level verified を返してはならない"
        );
        assert_eq!(report["implementation_conformance"]["status"], "pass");
        assert_eq!(
            report["implementation_conformance"]["method"],
            "sampled-property"
        );
        assert_eq!(report["implementation_conformance"]["cases"], 5);
        assert_eq!(
            report["implementation_conformance"]["coverage"]["executed"],
            5
        );
        assert_eq!(report["implementation_conformance"]["target"], "unknown");
        assert_eq!(report["intent_validation"]["status"], "unknown");
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_test_format_json_reports_vacuous_failure_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_test_format_json_vacuous_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [sample Int] :cases 1 :postcondition true)] x)",
    )
    .expect("vacuous JSON property fixture input.ls の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls", "--format", "json"])
            .output()
            .expect("native App.Cli vacuous test --format json の実行に失敗");
        assert_eq!(
            test.status.code(),
            Some(2),
            "native test --format json は vacuous property を exit 2 にするべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        let stdout = String::from_utf8_lossy(&test.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "native failure JSON は report 1 行を返すべき"
        );
        let report: Value =
            serde_json::from_str(lines[0]).expect("native failure JSON は valid JSON");
        assert_eq!(report["implementation_conformance"]["status"], "fail");
        assert_eq!(
            report["implementation_conformance"]["diagnostics"]["count"],
            1
        );
        assert_eq!(
            report["implementation_conformance"]["diagnostics"]["firstErrorCode"],
            2005
        );
        assert_eq!(report["intent_validation"]["status"], "unknown");
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_test_format_json_rejects_dynamic_complement_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_test_format_json_dynamic_complement_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [value Int] :cases 1 :postcondition (or (= value 0) (not (= value 0))))] x)",
    )
    .expect("dynamic complement JSON property fixture input.ls の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls", "--format", "json"])
            .output()
            .expect("native App.Cli dynamic complement test --format json の実行に失敗");
        assert_eq!(
            test.status.code(),
            Some(2),
            "native test --format json は dynamic complement property を exit 2 にするべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        let stdout = String::from_utf8_lossy(&test.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "dynamic complement failure JSON は report 1 行を返すべき"
        );
        let report: Value =
            serde_json::from_str(lines[0]).expect("dynamic complement failure JSON は valid JSON");
        assert_eq!(report["implementation_conformance"]["status"], "fail");
        assert_eq!(
            report["implementation_conformance"]["diagnostics"]["count"],
            1
        );
        assert_eq!(
            report["implementation_conformance"]["diagnostics"]["firstErrorCode"],
            2005
        );
        assert_eq!(report["intent_validation"]["status"], "unknown");
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_check_reports_vacuous_property_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_check_vacuous_property_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [value Int] :postcondition (or (= value 0) (not (= value 0))))] x)",
    )
    .expect("vacuous property check fixture input.ls の書き込みに失敗");

    let result = (|| {
        let check = Command::new(&program)
            .current_dir(&dir)
            .args(["check", "input.ls"])
            .output()
            .expect("native App.Cli vacuous property check の実行に失敗");
        assert_eq!(
            check.status.code(),
            Some(1),
            "native check は vacuous property を compile error にするべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&check.stdout),
            String::from_utf8_lossy(&check.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&check.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec![
                "Fn",
                "diagnostics:1,T0001@1:1,first-body:property predicate is vacuous",
            ],
            "native check は vacuous property の専用 diagnostics body を返すべき"
        );
        assert!(
            check.stderr.is_empty(),
            "native check stderr は空であるべき: {:?}",
            String::from_utf8_lossy(&check.stderr)
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native EmbeddedCli program を LSHARP_NATIVE_EMBEDDED_CLI_PROGRAM で指定する"]
fn test_native_embedded_cli_test_format_json_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_EMBEDDED_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_EMBEDDED_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native EmbeddedCli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native EmbeddedCli の絶対パス化に失敗");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/metadata.ls");
    let source = std::fs::canonicalize(&source).expect("metadata fixture の絶対パス化に失敗");

    let test = Command::new(&program)
        .current_dir(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("workspace root の解決に失敗"),
        )
        .args([
            "test",
            source.to_str().expect("metadata path は UTF-8 であるべき"),
            "--format",
            "json",
        ])
        .output()
        .expect("native EmbeddedCli test --format json の実行に失敗");

    assert!(
        test.status.success(),
        "native EmbeddedCli test --format json は成功するべき: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&test.stdout),
        String::from_utf8_lossy(&test.stderr)
    );
    let stdout = String::from_utf8_lossy(&test.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(
        lines.len(),
        1,
        "native EmbeddedCli test --format json は report 1 行を返すべき"
    );
    let report: Value =
        serde_json::from_str(lines[0]).expect("native EmbeddedCli JSON は valid JSON");
    assert!(
        report.get("verified").is_none(),
        "native EmbeddedCli assurance report は top-level verified を返してはならない"
    );
    assert_eq!(report["implementation_conformance"]["status"], "pass");
    assert_eq!(
        report["implementation_conformance"]["method"],
        "legacy-deterministic-smoke"
    );
    assert!(
        report["implementation_conformance"]["cases"]
            .as_u64()
            .unwrap_or(0)
            > 0
    );
    assert_eq!(report["intent_validation"]["status"], "unknown");
}

#[test]
#[ignore = "actual native EmbeddedCli program を LSHARP_NATIVE_EMBEDDED_CLI_PROGRAM で指定する"]
fn test_native_embedded_cli_test_format_json_reports_vacuous_failure_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_EMBEDDED_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_EMBEDDED_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native EmbeddedCli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native EmbeddedCli の絶対パス化に失敗");
    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_embedded_cli_test_format_json_vacuous_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("EmbeddedCli vacuous fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [sample Int] :cases 1 :postcondition true)] x)",
    )
    .expect("EmbeddedCli vacuous fixture の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls", "--format", "json"])
            .output()
            .expect("native EmbeddedCli vacuous test --format json の実行に失敗");
        assert_eq!(
            test.status.code(),
            Some(2),
            "native EmbeddedCli JSON failure は exit 2 であるべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        let stdout = String::from_utf8_lossy(&test.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        assert_eq!(
            lines.len(),
            1,
            "native EmbeddedCli JSON failure は report 1 行を返すべき"
        );
        let report: Value =
            serde_json::from_str(lines[0]).expect("native EmbeddedCli failure JSON は valid JSON");
        assert_eq!(report["implementation_conformance"]["status"], "fail");
        assert_eq!(
            report["implementation_conformance"]["diagnostics"]["count"],
            1
        );
        assert_eq!(
            report["implementation_conformance"]["diagnostics"]["firstErrorCode"],
            2005
        );
        assert_eq!(report["intent_validation"]["status"], "unknown");
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_test_rejects_vacuous_property_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_vacuous_property_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn identity [x] :property [(for-all [sample Int] :cases 1 :postcondition true)] x)",
    )
    .expect("vacuous property fixture input.ls の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls"])
            .output()
            .expect("native App.Cli vacuous property test の実行に失敗");
        assert_eq!(
            test.status.code(),
            Some(2),
            "native test は vacuous property を runtime error にするべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&test.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec![
                "examples:0",
                "invariants:0",
                "properties:1",
                "failures:1",
                "diagnostics:1,LS2005",
            ],
            "native test は vacuous property の診断付き summary を出力するべき"
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

#[test]
#[ignore = "actual native App.Cli program を LSHARP_NATIVE_APP_CLI_PROGRAM で指定する"]
fn test_native_app_cli_test_metadata_source_file_contract() {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return;
    }

    let program = PathBuf::from(
        std::env::var_os("LSHARP_NATIVE_APP_CLI_PROGRAM")
            .expect("LSHARP_NATIVE_APP_CLI_PROGRAM を指定すること"),
    );
    assert!(
        program.is_file(),
        "native App.Cli が見つからない: {}",
        program.display()
    );
    let program = std::fs::canonicalize(&program).expect("native App.Cli の絶対パス化に失敗");

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_metadata_test_contract_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(
        dir.join("input.ls"),
        "(defn abs [x] :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))",
    )
    .expect("metadata fixture input.ls の書き込みに失敗");

    let result = (|| {
        let test = Command::new(&program)
            .current_dir(&dir)
            .args(["test", "input.ls"])
            .output()
            .expect("native App.Cli metadata test の実行に失敗");
        assert!(
            test.status.success(),
            "native metadata test は成功するべき: stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&test.stdout),
            String::from_utf8_lossy(&test.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&test.stdout)
                .lines()
                .collect::<Vec<_>>(),
            vec!["examples:2", "invariants:1", "failures:0"],
            "native metadata test は passing suite の成功 summary を出力するべき"
        );
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}
