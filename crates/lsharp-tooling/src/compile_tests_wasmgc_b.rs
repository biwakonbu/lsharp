#[test]
fn test_compile_file_wasmgc_backend_executes_string_array_get() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-array-get");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (string-char-at \"hello\" 1))\n").unwrap();

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
    assert_eq!(main.call(&mut store, ()).unwrap(), 101);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_reads_utf8_byte_as_unsigned() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-packed-byte");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(&file, "(defn main [] (string-char-at \"é\" 0))\n").unwrap();

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
    assert_eq!(main.call(&mut store, ()).unwrap(), 195);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_passes_string_array_to_user_function() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-array-param");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(defn length [value] (string-length value))\n\
             (defn main [] (length \"hello\"))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 5);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_string_equality() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-eq");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(defn same [left right] (string-eq left right))\n\
             (defn main []\n\
               (+ (if (same \"hello\" \"hello\") 1 0)\n\
                  (+ (if (same \"hello\" \"world\") 0 2)\n\
                     (+ (if (same \"hi\" \"hello\") 0 4)\n\
                        (if (same \"\" \"\") 8 0)))))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 15);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_string_concat() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-concat");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(defn join [left right] (string-concat left right))\n\
             (defn main []\n\
               (+ (string-length (join \"hello\" \" world\"))\n\
                  (+ (string-char-at (join \"a\" \"b\") 1)\n\
                     (string-length (join \"\" \"\")))))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 109);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_string_substring() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-string-substring");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(defn slice [value start end] (substring value start end))\n\
             (defn main []\n\
               (+ (string-length (slice \"hello world\" 6 11))\n\
                  (+ (string-char-at (slice \"hello world\" 6 11) 1)\n\
                     (string-length (slice \"abc\" 1 1)))))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 116);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_traps_dynamic_invalid_substring_ranges() {
    let cases = [
        ("negative-start", "(- 0 1)", "1"),
        ("reversed-range", "2", "1"),
        ("end-overflow", "1", "4"),
    ];

    for (case_name, start, end) in cases {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("workspace tmp root")
            .join(format!(
                "lsharp-wasmgc-string-substring-invalid-{case_name}"
            ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();

        let file = dir.join("Main.ls");
        let output = dir.join("Main.wasm");
        std::fs::write(
            &file,
            format!(
                "(defn slice [value start end] (substring value start end))\n\
                     (defn main [] (string-length (slice \"abc\" {start} {end})))\n"
            ),
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
        let error = main
            .call(&mut store, ())
            .expect_err("invalid substring range は Wasm unreachable で止まるべき");
        assert!(
            matches!(
                error.downcast_ref::<wasmtime::Trap>(),
                Some(wasmtime::Trap::UnreachableCodeReached)
            ),
            "invalid substring range の trap が unreachable ではない: {error}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}

#[test]
fn test_wasmgc_backend_rejects_unsupported_record_string_literal_pattern() {
    let error = compile_module_from_formatted_source(
        Path::new("Main.ls"),
        "(type Point (record (: x String)))\n\
             (type Box (Box Point))\n\
             (defn read-point [value]\n\
               (match value [(Box {Point x \"value\"}) 1] [_ 0]))\n",
        CompileBackend::WasmGc,
    )
    .expect_err(
        "WasmGC backend は未対応の record literal pattern を暗黙に linear lowering してはならない",
    );
    assert!(error.to_string().contains("LS3001"));
    assert!(error.to_string().contains("nested/literal"));
}

#[test]
fn test_compile_file_wasmgc_backend_executes_integer_adt_literal_pattern() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_literal_adt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Maybe (Just Int) Nothing)\n\
             (type Flag (Set Bool) Off)\n\
             (defn is-forty-two [value]\n\
               (match value [(Just 42) 1] [_ 0]))\n\
             (defn is-true [value]\n\
               (match value [(Set true) 1] [_ 0]))\n\
             (defn main [] (+ (is-forty-two (Just 42))\
                              (+ (is-forty-two (Just 41)) (is-true (Set true)))))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 2);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_nested_adt_constructor_and_pattern() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_nested_adt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Maybe (Just Int) Nothing)\n\
             (type Box (Box Maybe))\n\
             (defn unwrap-box [value] (match value [(Box (Just x)) x] [_ 0]))\n\
             (defn main [] (+ (unwrap-box (Box (Just 42))) (unwrap-box (Box Nothing))))\n",
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
fn test_wasmgc_backend_rejects_unresolved_adt_payload_type() {
    let error = compile_module_from_formatted_source(
        Path::new("Main.ls"),
        "(type Box (Box String))\n(defn main [] (Box \"value\"))\n",
        CompileBackend::WasmGc,
    )
    .expect_err("WasmGC backend は未対応 payload を i64 に暗黙変換してはならない");

    assert!(error.to_string().contains("LS3001"));
    assert!(error.to_string().contains("payload"));
}

#[test]
fn test_compile_file_wasmgc_backend_preserves_nested_adt_binding_type() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_nested_adt_binding");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Maybe (Just Int) Nothing)\n\
             (type Box (Box Maybe))\n\
             (defn unwrap-box [value]\n\
               (match value [(Box inner) (match inner [(Just x) x] [_ 0])] [_ 0]))\n\
             (defn main [] (+ (unwrap-box (Box (Just 42))) (unwrap-box (Box Nothing))))\n",
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
fn test_compile_file_wasmgc_backend_executes_nullable_adt_payload() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_nullable_adt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Maybe (Just Int) Nothing)\n\
             (type MaybeBox (Present Maybe) Empty)\n\
             (defn unwrap [value]\n\
               (match value [(Present (Just x)) x] [_ 0]))\n\
             (defn main [] (+ (unwrap (Present (Just 42))) (unwrap (Present Nothing))))\n",
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
fn test_compile_file_wasmgc_backend_executes_nested_record_access() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_nested_record");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Inner (record (: x Int)))\n\
             (type Outer (record (: inner Inner)))\n\
             (defn main [] (. (. {Outer inner {Inner x 41}} inner) x))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 41);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_record_literal_pattern_with_fallback() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-record-pattern-red");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Point (record (: x Int) (: y Int)))\n\
             (defn classify [point]\n\
               (match point [{Point x 42 y value} value] [_ 0]))\n\
             (defn main [] (+ (classify {Point x 42 y 7})\
                              (classify {Point x 41 y 7})))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 7);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_nested_record_literal_pattern() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("workspace tmp root")
        .join("lsharp-wasmgc-nested-record-pattern");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Inner (record (: x Int)))\n\
             (type Outer (record (: inner Inner)))\n\
             (defn classify [outer]\n\
               (match outer [{Outer inner {Inner x 42}} 1] [_ 0]))\n\
             (defn main [] (+ (classify {Outer inner {Inner x 42}})\
                              (classify {Outer inner {Inner x 41}})))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 1);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_file_wasmgc_backend_executes_record_update() {
    let dir = std::env::temp_dir().join("lsharp_compile_pipeline_wasmgc_record_update");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".git")).unwrap();

    let file = dir.join("Main.ls");
    let output = dir.join("Main.wasm");
    std::fs::write(
        &file,
        "(type Point (record (: x Int) (: y Int)))\n\
             (defn main [] (let [p {Point x 10 y 20} q {p | x 42}] (. q y)))\n",
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
    assert_eq!(main.call(&mut store, ()).unwrap(), 20);

    std::fs::remove_dir_all(&dir).unwrap();
}
