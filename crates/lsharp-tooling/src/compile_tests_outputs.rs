#[test]
fn test_compile_file_wasmgc_backend_rejects_non_web_target() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_target");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let error = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiPreview1),
        CompileBackend::WasmGc,
    )
    .expect_err("WasmGC backend は未対応 target を受け入れてはならない");
    assert!(error.to_string().contains("[LS4001]"));
    assert!(error.to_string().contains("--target web-wasm"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_wasmgc_backend_rejects_file_imports_explicitly() {
    let error = compile_module_from_formatted_source(
        Path::new("Main.ls"),
        "(import Foo)\n(defn main [] 42)\n",
        CompileBackend::WasmGc,
    )
    .expect_err("WasmGC backend は未対応の file import を曖昧に処理してはならない");

    assert!(error.to_string().contains("[LS4001]"));
    assert!(error.to_string().contains("import"));
}

#[test]
fn test_compile_file_preview1_target_writes_runnable_core_wasm() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_preview1");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (print 42))\n").unwrap();

    let artifacts = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiPreview1),
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();
    let stdout = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes)
        .expect("preview1 target は preview1 runner で実行できる core Wasm を出力するべき");
    assert_eq!(stdout, "42\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_defaults_to_wasi_component_output_extension() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_default_component_target");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let artifacts = compile_file(&file, None, false, None).unwrap();
    assert_eq!(artifacts.output_path, dir.join("Main.component.wasm"));
    assert!(artifacts.output_path.exists());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasi_component_output_validates_as_component() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_component_validation");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.component.wasm");
    std::fs::write(&file, "(defn main [] (print 42))\n").unwrap();

    let artifacts = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiComponent),
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();
    let stdout = lsharp_wasm::wasi_runner::run_wasm_component(&wasm_bytes)
        .expect("wasi-component target は preview2 runner で実行できる component を出力するべき");
    assert_eq!(stdout, "42\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasi_component_executes_constrained_type_helpers() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_component_constrained");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.component.wasm");
    std::fs::write(
        &file,
        "(type-constrained Natural Int :constraints [(>= 0)])\n\
             (defn main [] (print 42))\n",
    )
    .unwrap();

    let artifacts = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiComponent),
    )
    .unwrap();
    let component_bytes = std::fs::read(&artifacts.output_path).unwrap();
    let stdout = lsharp_wasm::wasi_runner::run_wasm_component(&component_bytes)
        .expect("制約付き型 helper を含む component は validation と実行に成功するべき");
    assert_eq!(stdout, "42\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasi_component_executes_record_access() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_component_record");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.component.wasm");
    std::fs::write(
        &file,
        "(type Point (record (: x Int) (: y Int)))\n\
             (defn make-point [x y] {Point x x y y})\n\
             (defn main [] (print (Point.x (make-point 10 20))))\n",
    )
    .unwrap();

    let artifacts = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiComponent),
    )
    .unwrap();
    let component_bytes = std::fs::read(&artifacts.output_path).unwrap();
    let stdout = lsharp_wasm::wasi_runner::run_wasm_component(&component_bytes)
        .expect("record access を含む component は validation と実行に成功するべき");
    assert_eq!(stdout, "10\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_prepare_source_for_compile_rewrites_file_when_format_diff_exists() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_format");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let file = dir.join("Main.ls");
    std::fs::write(&file, "(defn   main  []   42)\n").unwrap();

    let (formatted, changed) = prepare_source_for_compile(&file).unwrap();
    let on_disk = std::fs::read_to_string(&file).unwrap();

    assert!(changed, "format 差分があるので changed=true を返すべき");
    assert_eq!(
        formatted, on_disk,
        "compile 前にフォーマット済みソースを書き戻すべき"
    );
    assert!(
        on_disk.contains("(defn main"),
        "compile 前に空白が正規化されるべき: {on_disk}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_prepare_source_for_compile_preserves_escaped_quotes_in_strings() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_escape_quotes");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let file = dir.join("Main.ls");
    std::fs::write(
            &file,
            "(defn   main  []   (print \"\\\"id\\\":\"))\n(defn parse [] (print \"\\\"method\\\":\\\"initialize\\\"\"))\n",
        )
        .unwrap();

    let (formatted, changed) = prepare_source_for_compile(&file).unwrap();
    let on_disk = std::fs::read_to_string(&file).unwrap();

    assert!(changed, "format 差分があるので changed=true を返すべき");
    assert_eq!(formatted, on_disk, "compile 前に書き戻した内容を返すべき");
    assert!(
        formatted.contains("\"\\\"id\\\":\""),
        "escaped quote を含む文字列リテラルが壊れている: {formatted}"
    );
    assert!(
        formatted.contains("\"\\\"method\\\":\\\"initialize\\\"\""),
        "escaped quote を含む method 文字列が壊れている: {formatted}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_runs_format_check_codegen_pipeline() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_codegen");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.component.wasm");
    std::fs::write(&file, "(defn   main  []   42)\n").unwrap();

    let artifacts = compile_file(&file, Some(&output), false, None).unwrap();

    assert_eq!(artifacts.output_path, output);
    assert!(
        artifacts.formatted,
        "compile は format 差分を検出して書き戻すべき"
    );
    assert!(output.exists(), "compile は Wasm 出力を生成するべき");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_web_wasm_target_uses_core_codegen_path() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_web_wasm");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let artifacts =
        compile_file(&file, Some(&output), false, Some(CompileTarget::WebWasm)).unwrap();
    assert_eq!(artifacts.output_path, output);

    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();
    assert!(
        wasm_bytes
            .windows(b"env".len())
            .any(|window| window == b"env"),
        "web-wasm 出力には env import 名が含まれるべき"
    );
    assert!(
        !wasm_bytes
            .windows(b"wasi_snapshot_preview1".len())
            .any(|window| window == b"wasi_snapshot_preview1"),
        "web-wasm は preview1 import 名を含むべきではない"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_plain_wasm_output_without_target_keeps_wasi_codegen() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_plain_wasm_default");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let artifacts = compile_file(&file, Some(&output), false, None).unwrap();
    assert_eq!(artifacts.output_path, output);

    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();
    assert!(
        wasm_bytes
            .windows(b"wasi_snapshot_preview1".len())
            .any(|window| window == b"wasi_snapshot_preview1"),
        "plain .wasm output は後方互換のため preview1 import を維持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_handle_only_emits_http_handler_component_export() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_http_handler_component");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Handler.ls");
    let output = dir.join("Handler.component.wasm");
    std::fs::write(&file, r#"(defn handle [request] "ok")"#).unwrap();

    let artifacts = compile_file(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WasiComponent),
    )
    .unwrap();
    let component_bytes = std::fs::read(&artifacts.output_path).unwrap();
    let engine = wasmtime::Engine::default();
    let component = wasmtime::component::Component::new(&engine, &component_bytes)
        .expect("HTTP handler source should compile into a valid component");

    assert!(
        component
            .export_index(None, "wasi:http/incoming-handler@0.2.3")
            .is_some(),
        "handle-only source should emit HTTP handler world exports"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_executes_print_i64_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_print");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(&file, "(defn main [] (print 42))\n").unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let output = std::process::Command::new(&artifacts.output_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
    assert!(output.stderr.is_empty());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_executes_user_function_call_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_call");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(
        &file,
        "(defn double [x] (+ x x))\n(defn main [] (double 21))\n",
    )
    .unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let status = std::process::Command::new(&artifacts.output_path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(42));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_ignores_unreachable_runtime_helpers_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_reachable");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(
            &file,
            "(type (Maybe a) (Just a) Nothing)\n(defn identity [x] x)\n(defn main [] (print (identity 42)))\n",
        )
        .unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let output = std::process::Command::new(&artifacts.output_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n");
    assert!(output.stderr.is_empty());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_executes_record_access_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_record");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(
        &file,
        "(type Point (record (: x Int) (: y Int)))\n\
             (defn make-point [x y] {Point x x y y})\n\
             (defn get-x [p] (Point.x p))\n\
             (defn main [] (let [p (make-point 10 20)] (print (get-x p))))\n",
    )
    .unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let output = std::process::Command::new(&artifacts.output_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "10\n");
    assert!(output.stderr.is_empty());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_executes_adt_match_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_adt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(
        &file,
        "(type (Option a) (Some a) None)\n\
             (defn unwrap-or [(: opt (Option Int)) (: default Int)] : Int\n\
               (match opt [(Some x) x] [None default]))\n\
             (defn main []\n\
               (let [x (Some 42) y None]\n\
                 (do (print (unwrap-or x 0)) (print (unwrap-or y 0)))))\n",
    )
    .unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let output = std::process::Command::new(&artifacts.output_path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "42\n0\n");
    assert!(output.stderr.is_empty());

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_executes_recursive_if_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_fib");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(
            &file,
            "(defn fib [n] (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2)))))\n(defn main [] (+ (fib 8) 21))\n",
        )
        .unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let status = std::process::Command::new(&artifacts.output_path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(42));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_executes_simple_i64_arithmetic_aarch64_macos() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_arithmetic");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(&file, "(defn main [] (+ 40 2))\n").unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    let status = std::process::Command::new(&artifacts.output_path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(42));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn test_compile_file_native_target_writes_runnable_aarch64_macos_binary() {
    use std::os::unix::fs::PermissionsExt;

    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
    assert_eq!(artifacts.output_path, output);
    assert!(
        artifacts.output_path.exists(),
        "native binary を生成するべき"
    );

    let mode = std::fs::metadata(&artifacts.output_path)
        .unwrap()
        .permissions()
        .mode();
    assert!(
        mode & 0o111 != 0,
        "native binary は実行可能であるべき: mode={mode:o}"
    );

    let status = std::process::Command::new(&artifacts.output_path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(42));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn compile_file_missing_source_preserves_driver_io_error_code() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_missing_source_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Missing.ls");
    let output = dir.join("missing.wasm");

    let error = compile_file(&file, Some(&output), false, Some(CompileTarget::WebWasm))
        .expect_err("存在しない source file は compile を失敗させるべき");
    assert!(
        error.to_string().starts_with("[LS5001]"),
        "driver I/O error code が必要: {error}"
    );
    assert!(error.to_string().contains("Missing.ls"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn compile_file_artifact_write_failure_preserves_driver_io_error_code() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_artifact_write_failure_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Main.ls");
    let output = dir.join("missing-parent").join("main.wasm");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let error = compile_file(&file, Some(&output), false, Some(CompileTarget::WebWasm))
        .expect_err("存在しない artifact parent は compile を失敗させるべき");
    assert!(
        error.to_string().starts_with("[LS5001]"),
        "driver I/O error code が必要: {error}"
    );
    assert!(error.to_string().contains("main.wasm"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn test_compile_file_native_target_returns_explicit_error() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_native");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("demo");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let err = compile_file(&file, Some(&output), false, None).unwrap_err();
    let message = err.to_string();
    assert!(
        message.contains("[LS4001]"),
        "native target の診断コードが必要: {message}"
    );
    assert!(
        message.contains("native backend は未サポート"),
        "native target の明示エラーが必要: {message}"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
