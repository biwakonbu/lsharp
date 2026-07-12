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
