use std::{path::PathBuf, process::Command};

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

    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_app_cli_actual_output_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("fixture directory の作成に失敗");
    std::fs::write(dir.join("input.ls"), "(defn main [] 42)")
        .expect("fixture input.ls の書き込みに失敗");

    let result = (|| {
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
            let artifact = std::fs::read(dir.join(&output_name))
                .unwrap_or_else(|_| panic!("native {command} output の読み込みに失敗"));
            assert!(
                artifact.starts_with(b"\0asm"),
                "native {command} -o は summary text ではなく actual Wasm を書くべき: {:?}",
                artifact
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
