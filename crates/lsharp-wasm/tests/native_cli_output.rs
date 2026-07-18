use std::{path::PathBuf, process::Command};

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
