
#[test]
#[ignore]
fn test_debug_boot04_stage2_build_compile_progress_on_minimal_path_parent_shape() {
    let temp_root = selfhost_project_root()
        .join("target/test-artifacts")
        .join(format!(
            "lsharp_path_parent_minimal_build_progress_{}",
            std::process::id()
        ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_path_parent_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.ModuleResolver)\n(defn path-parent [path] (let [len (string-length path)] (if (= len 0) \"\" (if (has-path-sep path 0 len) (let [last (find-last-path-sep path 0 len -1)] (if (< last 0) \"\" (if (= last 0) \"/\" (substring path 0 last)))) \".\"))))\n(defn path-char [path idx] (string-char-at path idx))\n(defn is-path-sep [path idx] (let [ch (path-char path idx)] (if (= ch 47) true (if (= ch 92) true false))))\n(defn has-path-sep [path idx len] (if (>= idx len) false (if (is-path-sep path idx) true (has-path-sep path (+ idx 1) len))))\n(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))\n(defn main [] (print (string-length (path-parent (command-line-arg 1)))))\n",
    )
    .expect("mini source should be written");

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let stage2_output = lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir_and_args(
        &stage1_wasm,
        Some(&selfhost_root),
        &["compiler", "src/App/Main.ls"],
    )
    .expect("stage1 should self-compile Main.ls");
    let stage2_modules = parse_emitted_wasm_modules(&stage2_output, 1);
    let stage2_self_compiler = &stage2_modules[0];
    assert_valid_wasm(stage2_self_compiler);

    let source_path_str = source_path.to_str().expect("utf-8 path");
    let progress_output = run_wasm_with_eleven_imports_compiler_mode_fs(
        stage2_self_compiler,
        &selfhost_root,
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage2 build compile progress on minimal path-parent source should run");
    eprintln!(
        "BOOT-04 minimal path-parent build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_app_cli() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/App/Cli.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on App/Cli.ls should run");
    eprintln!(
        "BOOT-04 App/Cli stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_compiler_module() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/Backend/Wasm/Compiler.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on Compiler.ls should run");
    eprintln!(
        "BOOT-04 Compiler.ls stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_compiler_mode_module() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            "src/App/CompilerMode.ls",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on CompilerMode.ls should run");
    eprintln!(
        "BOOT-04 CompilerMode.ls stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_minimal_vector_push_shape() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let temp_root = selfhost_root.join("target/test-artifacts").join(format!(
        "lsharp_vector_push_minimal_build_progress_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("mini_vector_push_shape.ls");
    std::fs::write(
        &source_path,
        "(module Mini.Compiler)\n(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (do (root_push node) (root_push source) (root_push env) (root_push ftable) (root_push instrs) (root_push data-ref) (let [vector-root (alloc-root-needed (vector-get node 3)) vector-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)] (do (root_push vector-instrs) (let [value-root (alloc-root-needed (vector-get node 4)) value-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)] (do (root_push value-instrs) (let [temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) instrs1 (append-instr-vector instrs vector-instrs)] (do (root_push instrs1) (let [instrs2 (emit-to instrs1 (op-local-set) vector-local)] (do (root_push instrs2) (let [instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)] (do (root_push instrs3) (let [instrs4 (append-instr-vector instrs3 value-instrs)] (do (root_push instrs4) (let [instrs5 (emit-to instrs4 (op-local-set) value-local)] (do (root_push instrs5) (let [instrs6 (maybe-root-push-drop instrs5 value-root value-local)] (do (root_push instrs6) (let [instrs7 (emit-to instrs6 (op-local-get) vector-local)] (do (root_push instrs7) (let [instrs8 (emit-to instrs7 (op-local-get) value-local)] (do (root_push instrs8) (let [instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env)))] (do (root_push instrs9) (let [instrs10 (maybe-root-pop-drop instrs9 value-root)] (do (root_push instrs10) (let [result (maybe-root-pop-drop instrs10 vector-root)] (do (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) result))))))))))))))))))))))))))))\n(defn root_push [x] 0)\n(defn root_pop [] 0)\n(defn alloc-root-needed [x] 0)\n(defn vector-get [v idx] 0)\n(defn vector-new [n] 0)\n(defn compile-expr-with-source [expr source env ftable instrs data-ref] instrs)\n(defn max-root-temp-base [env lhs rhs] 0)\n(defn append-instr-vector [lhs rhs] lhs)\n(defn emit-to [instrs op arg] instrs)\n(defn maybe-root-push-drop [instrs should-root local-idx] instrs)\n(defn maybe-root-pop-drop [instrs should-root] instrs)\n(defn op-local-set [] 0)\n(defn op-local-get [] 0)\n(defn op-vector-push [] 0)\n(defn map-size [m] 0)\n(defn main [] 0)\n",
    )
    .expect("mini source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("source path should stay under selfhost root")
        .to_str()
        .expect("utf-8 path");
    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on minimal vector-push source should run");
    eprintln!(
        "BOOT-04 minimal vector-push stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_padded_vector_push_shape() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let temp_root = selfhost_root.join("target/test-artifacts").join(format!(
        "lsharp_vector_push_padded_build_progress_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("padded_vector_push_shape.ls");

    let filler = "x".repeat(700);
    let mut source = String::from("(module Mini.Compiler)\n");
    for i in 0..198 {
        source.push_str(&format!("(defn filler{i:03} [] \"{filler}\")\n"));
    }
    source.push_str("(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (do (root_push node) (root_push source) (root_push env) (root_push ftable) (root_push instrs) (root_push data-ref) (let [vector-root (alloc-root-needed (vector-get node 3)) vector-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)] (do (root_push vector-instrs) (let [value-root (alloc-root-needed (vector-get node 4)) value-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)] (do (root_push value-instrs) (let [temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) instrs1 (append-instr-vector instrs vector-instrs)] (do (root_push instrs1) (let [instrs2 (emit-to instrs1 (op-local-set) vector-local)] (do (root_push instrs2) (let [instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)] (do (root_push instrs3) (let [instrs4 (append-instr-vector instrs3 value-instrs)] (do (root_push instrs4) (let [instrs5 (emit-to instrs4 (op-local-set) value-local)] (do (root_push instrs5) (let [instrs6 (maybe-root-push-drop instrs5 value-root value-local)] (do (root_push instrs6) (let [instrs7 (emit-to instrs6 (op-local-get) vector-local)] (do (root_push instrs7) (let [instrs8 (emit-to instrs7 (op-local-get) value-local)] (do (root_push instrs8) (let [instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env)))] (do (root_push instrs9) (let [instrs10 (maybe-root-pop-drop instrs9 value-root)] (do (root_push instrs10) (let [result (maybe-root-pop-drop instrs10 vector-root)] (do (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) result))))))))))))))))))))))))))))\n");
    source.push_str("(defn root_push [x] 0)\n(defn root_pop [] 0)\n(defn alloc-root-needed [x] 0)\n(defn vector-get [v idx] 0)\n(defn vector-new [n] 0)\n(defn compile-expr-with-source [expr source env ftable instrs data-ref] instrs)\n(defn max-root-temp-base [env lhs rhs] 0)\n(defn append-instr-vector [lhs rhs] lhs)\n(defn emit-to [instrs op arg] instrs)\n(defn maybe-root-push-drop [instrs should-root local-idx] instrs)\n(defn maybe-root-pop-drop [instrs should-root] instrs)\n(defn op-local-set [] 0)\n(defn op-local-get [] 0)\n(defn op-vector-push [] 0)\n(defn map-size [m] 0)\n(defn main [] 0)\n");
    std::fs::write(&source_path, source).expect("padded source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("source path should stay under selfhost root")
        .to_str()
        .expect("utf-8 path");
    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on padded vector-push source should run");
    eprintln!(
        "BOOT-04 padded vector-push stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}

#[test]
#[ignore]
fn test_debug_boot04_stage1_build_compile_progress_on_large_ftable_vector_push_shape() {
    fn run_wasi_capture_trap_stdout(
        wasm_bytes: &[u8],
        dir: Option<&std::path::Path>,
        args: &[&str],
    ) -> Result<String, String> {
        use wasmtime::{Engine, Linker, Module, Store};
        use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

        let engine = Engine::default();
        let mut linker = Linker::<WasiP1Ctx>::new(&engine);
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx| ctx)
            .map_err(|e| format!("WASI リンクに失敗: {e}"))?;

        let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4 * 1024 * 1024);
        let mut builder = WasiCtxBuilder::new();
        builder.stdout(stdout.clone());
        builder.args(args);
        if let Some(dir_path) = dir {
            builder
                .preopened_dir(
                    dir_path,
                    ".",
                    wasmtime_wasi::DirPerms::all(),
                    wasmtime_wasi::FilePerms::all(),
                )
                .map_err(|e| format!("preopened_dir に失敗: {e}"))?;
        }
        let mut store = Store::new(&engine, builder.build_p1());
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("Wasm モジュールの読み込みに失敗: {e}"))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| format!("インスタンス化に失敗: {e}"))?;
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|e| format!("_start 関数が見つからない: {e}"))?;
        let result = start.call(&mut store, ());
        drop(store);
        let bytes = stdout
            .try_into_inner()
            .ok_or_else(|| "stdout の取得に失敗".to_string())?;
        let printed = String::from_utf8(bytes.to_vec())
            .map_err(|e| format!("stdout の UTF-8 変換に失敗: {e}"))?;
        match result {
            Ok(()) => Ok(printed),
            Err(e) => Err(format!("実行に失敗: {e}; printed={printed:?}")),
        }
    }

    let main_path = selfhost_main_path();
    let selfhost_root = main_path
        .parent()
        .expect("App/ ディレクトリ")
        .parent()
        .expect("src/ ディレクトリ")
        .parent()
        .expect("selfhost/ ルートディレクトリ")
        .to_path_buf();
    let temp_root = selfhost_root.join("target/test-artifacts").join(format!(
        "lsharp_vector_push_large_ftable_build_progress_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    std::fs::create_dir_all(&temp_root).expect("temp dir should be created");
    let source_path = temp_root.join("large_ftable_vector_push_shape.ls");

    let mut source = String::from("(module Mini.Compiler)\n");
    for i in 0..198 {
        source.push_str(&format!("(defn prefix{i:03} [] {i})\n"));
    }
    source.push_str("(defn compile-vector-push-with-source [node source env ftable instrs data-ref] (do (root_push node) (root_push source) (root_push env) (root_push ftable) (root_push instrs) (root_push data-ref) (let [vector-root (alloc-root-needed (vector-get node 3)) vector-instrs (compile-expr-with-source (vector-get node 3) source env ftable (vector-new 8) data-ref)] (do (root_push vector-instrs) (let [value-root (alloc-root-needed (vector-get node 4)) value-instrs (compile-expr-with-source (vector-get node 4) source env ftable (vector-new 8) data-ref)] (do (root_push value-instrs) (let [temp-base (max-root-temp-base env vector-instrs value-instrs) vector-local temp-base value-local (+ temp-base 1) instrs1 (append-instr-vector instrs vector-instrs)] (do (root_push instrs1) (let [instrs2 (emit-to instrs1 (op-local-set) vector-local)] (do (root_push instrs2) (let [instrs3 (maybe-root-push-drop instrs2 vector-root vector-local)] (do (root_push instrs3) (let [instrs4 (append-instr-vector instrs3 value-instrs)] (do (root_push instrs4) (let [instrs5 (emit-to instrs4 (op-local-set) value-local)] (do (root_push instrs5) (let [instrs6 (maybe-root-push-drop instrs5 value-root value-local)] (do (root_push instrs6) (let [instrs7 (emit-to instrs6 (op-local-get) vector-local)] (do (root_push instrs7) (let [instrs8 (emit-to instrs7 (op-local-get) value-local)] (do (root_push instrs8) (let [instrs9 (emit-to instrs8 (op-vector-push) (+ 1 (map-size env)))] (do (root_push instrs9) (let [instrs10 (maybe-root-pop-drop instrs9 value-root)] (do (root_push instrs10) (let [result (maybe-root-pop-drop instrs10 vector-root)] (do (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) (root_pop) result))))))))))))))))))))))))))))\n");
    source.push_str("(defn root_push [x] 0)\n(defn root_pop [] 0)\n(defn alloc-root-needed [x] 0)\n(defn vector-get [v idx] 0)\n(defn vector-new [n] 0)\n(defn compile-expr-with-source [expr source env ftable instrs data-ref] instrs)\n(defn max-root-temp-base [env lhs rhs] 0)\n(defn append-instr-vector [lhs rhs] lhs)\n(defn emit-to [instrs op arg] instrs)\n(defn maybe-root-push-drop [instrs should-root local-idx] instrs)\n(defn maybe-root-pop-drop [instrs should-root] instrs)\n(defn op-local-set [] 0)\n(defn op-local-get [] 0)\n(defn op-vector-push [] 0)\n(defn map-size [m] 0)\n");
    for i in 0..800 {
        source.push_str(&format!("(defn suffix{i:03} [] {i})\n"));
    }
    source.push_str("(defn main [] 0)\n");
    std::fs::write(&source_path, source).expect("large ftable source should be written");

    let stage1_wasm = compile_file_only(&main_path);
    assert_valid_wasm(&stage1_wasm);

    let source_path_str = source_path
        .strip_prefix(&selfhost_root)
        .expect("source path should stay under selfhost root")
        .to_str()
        .expect("utf-8 path");
    let progress_output = run_wasi_capture_trap_stdout(
        &stage1_wasm,
        Some(&selfhost_root),
        &[
            "compiler",
            source_path_str,
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "build-compile-progress",
        ],
    )
    .expect("stage1 build compile progress on large-ftable vector-push source should run");
    eprintln!(
        "BOOT-04 large-ftable vector-push stage1 build compile progress = {:?}",
        progress_output
    );
    assert!(!progress_output.trim().is_empty());
}
