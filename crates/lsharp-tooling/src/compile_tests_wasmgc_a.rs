#[test]
fn test_resolve_compile_target_uses_output_extension_when_flag_missing() {
    let component_output = Path::new("demo.component.wasm");
    let wasm_output = Path::new("demo.wasm");
    let native_output = Path::new("demo");

    let (component_target, component_path) =
        resolve_compile_target(Some(component_output), None).unwrap();
    let (wasm_target, wasm_path) = resolve_compile_target(Some(wasm_output), None).unwrap();
    let (native_target, native_path) = resolve_compile_target(Some(native_output), None).unwrap();

    assert_eq!(component_target, CompileTarget::WasiComponent);
    assert_eq!(component_path, component_output);
    assert_eq!(wasm_target, CompileTarget::WasiPreview1);
    assert_eq!(wasm_path, wasm_output);
    assert_eq!(native_target, CompileTarget::Native);
    assert_eq!(native_path, native_output);
}

#[test]
fn test_resolve_compile_target_prefers_explicit_flag() {
    let output = Path::new("demo.component.wasm");
    let (target, resolved_path) =
        resolve_compile_target(Some(output), Some(CompileTarget::WebWasm)).unwrap();

    assert_eq!(target, CompileTarget::WebWasm);
    assert_eq!(resolved_path, output);
}

#[test]
fn test_compile_file_wasmgc_backend_writes_executable_core_wasm() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_backend");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 42);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_uses_atomic_artifact_boundary() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_pipeline_wasmgc_atomic_failure_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("WasmGC atomic failure directory を作成できる");
    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] 42)\n").unwrap();
    std::fs::create_dir(&output).expect("置換失敗を誘発する destination directory を作成できる");

    let error = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .expect_err("WasmGC artifact の directory 置換は失敗するべき");
    let message = error.to_string();
    assert!(
        message.contains("Wasm artifact の置換"),
        "WasmGC output は共有 atomic artifact writer の境界を通るべき: {message}"
    );
    assert!(
        output.is_dir(),
        "置換失敗時も既存 destination を保持するべき"
    );
    let temporary_residue = std::fs::read_dir(&dir)
        .expect("WasmGC atomic failure directory を列挙できる")
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".Main.wasm.tmp-")
        });
    assert!(
        !temporary_residue,
        "WasmGC atomic writer は一時 artifact を残さないべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_record_access() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_record");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Point (record (: x Int) (: y Int)))\n\
             (defn make-point [x y] {Point x x y y})\n\
             (defn main [] (Point.x (make-point 10 20)))\n",
    )
    .unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 10);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_adt_constructor_and_match() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_adt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Maybe (Just Int) Nothing)\n\
             (defn unwrap [value] (match value [(Just x) x] [Nothing 0]))\n\
             (defn main [] (+ (unwrap (Just 42)) (unwrap Nothing)))\n",
    )
    .unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 42);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_type_application_payload() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-type-application");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type (Inner a)\n\
               (Value Int)\n\
               Empty)\n\
             (type (Wrapper a)\n\
               (Wrapped (Inner Int))\n\
               EmptyWrapper)\n\
             (defn unwrap [wrapper]\n\
               (match wrapper\n\
                 [(Wrapped (Value value)) value]\n\
                 [_ 0]))\n\
             (defn main [] (unwrap (Wrapped (Value 42))))\n",
    )
    .unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 42);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_wasmgc_backend_rejects_recursive_type_application_payload_explicitly() {
    let error = compile_module_from_formatted_source(
        Path::new("Main.ls"),
        "(type (Expr a)\n\
               (Loop (Expr Int))\n\
               Halt)\n\
             (defn main [] Halt)\n",
        CompileBackend::WasmGc,
    )
    .expect_err("WasmGC は未検証の自己参照 GC payload を暗黙に実行してはならない");
    assert!(error.to_string().contains("LS3001"));
    assert!(error.to_string().contains("自己参照"));
}

#[test]
fn test_compile_file_wasmgc_backend_executes_scalar_gadt_refinement() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-gadt-scalar-red");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type (Expr a)\n\
               (: (IntLit Int) (Expr Int))\n\
               (: (BoolLit Bool) (Expr Bool)))\n\
             (defn get-int [expr]\n\
               (match expr\n\
                 [(IntLit value) value]\n\
                 [_ 0]))\n\
             (defn get-bool [expr]\n\
               (match expr\n\
                 [(BoolLit true) 1]\n\
                 [_ 0]))\n\
             (defn main [] (+ (get-int (IntLit 42)) (get-bool (BoolLit true))))\n",
    )
    .unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 43);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_computation_return() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-computation-return");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(defn bind-pass [value continuation] value)\n\
             (defn add-one [value] (+ value 1))\n\
             (computation-builder maybe-builder bind-pass add-one)\n\
             (defn main [] (computation maybe-builder (return 41)))\n",
    )
    .unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 42);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_wasmgc_backend_rejects_computation_bind_without_gc_closure() {
    let error = compile_module_from_formatted_source(
        Path::new("Main.ls"),
        "(defn bind-pass [value continuation] value)\n\
             (defn add-one [value] (+ value 1))\n\
             (computation-builder maybe-builder bind-pass add-one)\n\
             (defn main []\n\
               (computation maybe-builder\n\
                 (let! value 41)\n\
                 (return (add-one value))))\n",
        CompileBackend::WasmGc,
    )
    .expect_err("WasmGC は GC closure 未対応の computation bind を暗黙に直列評価してはならない");
    assert!(error.to_string().contains("LS3001"));
    assert!(error.to_string().contains("computation"));
    assert!(error.to_string().contains("closure"));
}

#[test]
fn test_compile_file_wasmgc_backend_executes_string_array_length() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-array-length");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (string-length \"hello\"))\n").unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 5);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_emits_print_string_import() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-print-import");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (do (print-string \"hello\") 0))\n").unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let import = module
        .imports()
        .next()
        .expect("print-string import が materialize される");
    assert_eq!(import.module(), "env");
    assert_eq!(import.name(), "print-string");

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_reads_print_string_with_host_import() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_print_string_host_read");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (do (print-string \"é\") 0))\n").unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();

    let mut config = wasmtime::Config::new();
    config.wasm_gc(true);
    let engine = wasmtime::Engine::new(&config).unwrap();
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let mut store = wasmtime::Store::new(&engine, ());
    let import = module
        .imports()
        .next()
        .expect("print-string import が materialize される");
    let wasmtime::ExternType::Func(func_type) = import.ty() else {
        panic!("print-string import は function であるべき");
    };
    let printed = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
    let printed_for_host = std::sync::Arc::clone(&printed);
    let print_string = lsharp_wasm::wasmgc_host::create_print_string_import(
        &mut store,
        func_type.clone(),
        move |bytes| {
            printed_for_host.lock().unwrap().push(bytes.to_vec());
            Ok(())
        },
    )
    .unwrap();
    let instance = wasmtime::Instance::new(&mut store, &module, &[print_string.into()])
        .expect("print-string host import を解決できる");
    let main = instance
        .get_typed_func::<(), i64>(&mut store, "main")
        .unwrap();
    assert_eq!(main.call(&mut store, ()).unwrap(), 0);
    assert_eq!(*printed.lock().unwrap(), vec![vec![195, 169]]);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_runs_with_public_runner_stdout_sink() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_runner_stdout");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (do (print-string \"é\") 7))\n").unwrap();

    let artifacts = compile_file_with_backend(
        &file,
        Some(&output),
        false,
        Some(CompileTarget::WebWasm),
        CompileBackend::WasmGc,
    )
    .unwrap();
    let wasm_bytes = std::fs::read(&artifacts.output_path).unwrap();
    let execution = lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_capture(&wasm_bytes)
        .expect("公開 WasmGC runner が source compile artifact を実行できる");

    assert_eq!(execution.stdout, "é");
    assert_eq!(execution.exit_code, 7);

    std::fs::remove_dir_all(&dir).unwrap();
}
