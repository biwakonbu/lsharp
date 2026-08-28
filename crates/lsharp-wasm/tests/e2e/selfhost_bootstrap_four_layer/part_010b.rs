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
    let values = parse_progress_values(&progress_output, "BOOT-04 minimal vector-push stage1 build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 minimal vector-push stage1 build compile progress");

    // import を持たない単一 fixture なので、先頭 pair と末尾 pair は同じものを指す。
    assert_eq!(values[2], 1, "BOOT-04 minimal vector-push stage1 build compile progress: import 無しなので pair 数は 1");
    assert_eq!(
        first_bytes, last_bytes,
        "BOOT-04 minimal vector-push stage1 build compile progress: 先頭と末尾の src バイト数が一致しない"
    );
    assert_eq!(
        first_decls, last_decls,
        "BOOT-04 minimal vector-push stage1 build compile progress: 先頭と末尾の decl 数が一致しない"
    );

    // fixture は本 test 自身が書いたものなので、compiler が読んだバイト数は実サイズと一致するはず。
    // 食い違うなら compiler が読んだのは別のファイルである。
    let written_bytes = std::fs::metadata(&source_path)
        .expect("BOOT-04 minimal vector-push stage1 build compile progress: fixture の metadata が取れない")
        .len() as i64;
    assert_eq!(
        last_bytes, written_bytes,
        "BOOT-04 minimal vector-push stage1 build compile progress: compiler が読んだバイト数が fixture の実サイズと違う"
    );

    // (module ...) は関数にならないので、生成関数は decl 数 - 1。
    assert_eq!(
        total_functions,
        last_decls - 1,
        "BOOT-04 minimal vector-push stage1 build compile progress: 生成関数の総数が decl 数 - 1 でない"
    );
    // pair ループは decl 1 個あたり 10 値、前後に 12 + 6 値が付く。
    assert_eq!(
        values.len() as i64,
        18 + 10 * (last_decls - 1),
        "BOOT-04 minimal vector-push stage1 build compile progress: progress の長さが decl 数から決まる形になっていない"
    );
    // fixture は test 内のリテラル / 生成ループから決まるので decl 数は exact に固定できる。
    assert_eq!(last_decls, 18, "BOOT-04 minimal vector-push stage1 build compile progress: fixture の decl 数が変わった (実測 18 / 2026-08-27)");
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
    let values = parse_progress_values(&progress_output, "BOOT-04 padded vector-push stage1 build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 padded vector-push stage1 build compile progress");

    // import を持たない単一 fixture なので、先頭 pair と末尾 pair は同じものを指す。
    assert_eq!(values[2], 1, "BOOT-04 padded vector-push stage1 build compile progress: import 無しなので pair 数は 1");
    assert_eq!(
        first_bytes, last_bytes,
        "BOOT-04 padded vector-push stage1 build compile progress: 先頭と末尾の src バイト数が一致しない"
    );
    assert_eq!(
        first_decls, last_decls,
        "BOOT-04 padded vector-push stage1 build compile progress: 先頭と末尾の decl 数が一致しない"
    );

    // fixture は本 test 自身が書いたものなので、compiler が読んだバイト数は実サイズと一致するはず。
    // 食い違うなら compiler が読んだのは別のファイルである。
    let written_bytes = std::fs::metadata(&source_path)
        .expect("BOOT-04 padded vector-push stage1 build compile progress: fixture の metadata が取れない")
        .len() as i64;
    assert_eq!(
        last_bytes, written_bytes,
        "BOOT-04 padded vector-push stage1 build compile progress: compiler が読んだバイト数が fixture の実サイズと違う"
    );

    // (module ...) は関数にならないので、生成関数は decl 数 - 1。
    assert_eq!(
        total_functions,
        last_decls - 1,
        "BOOT-04 padded vector-push stage1 build compile progress: 生成関数の総数が decl 数 - 1 でない"
    );
    // pair ループは decl 1 個あたり 10 値、前後に 12 + 6 値が付く。
    assert_eq!(
        values.len() as i64,
        18 + 10 * (last_decls - 1),
        "BOOT-04 padded vector-push stage1 build compile progress: progress の長さが decl 数から決まる形になっていない"
    );
    // fixture は test 内のリテラル / 生成ループから決まるので decl 数は exact に固定できる。
    assert_eq!(last_decls, 216, "BOOT-04 padded vector-push stage1 build compile progress: fixture の decl 数が変わった (実測 216 / 2026-08-27)");
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
    let values = parse_progress_values(&progress_output, "BOOT-04 large-ftable vector-push stage1 build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 large-ftable vector-push stage1 build compile progress");

    // import を持たない単一 fixture なので、先頭 pair と末尾 pair は同じものを指す。
    assert_eq!(values[2], 1, "BOOT-04 large-ftable vector-push stage1 build compile progress: import 無しなので pair 数は 1");
    assert_eq!(
        first_bytes, last_bytes,
        "BOOT-04 large-ftable vector-push stage1 build compile progress: 先頭と末尾の src バイト数が一致しない"
    );
    assert_eq!(
        first_decls, last_decls,
        "BOOT-04 large-ftable vector-push stage1 build compile progress: 先頭と末尾の decl 数が一致しない"
    );

    // fixture は本 test 自身が書いたものなので、compiler が読んだバイト数は実サイズと一致するはず。
    // 食い違うなら compiler が読んだのは別のファイルである。
    let written_bytes = std::fs::metadata(&source_path)
        .expect("BOOT-04 large-ftable vector-push stage1 build compile progress: fixture の metadata が取れない")
        .len() as i64;
    assert_eq!(
        last_bytes, written_bytes,
        "BOOT-04 large-ftable vector-push stage1 build compile progress: compiler が読んだバイト数が fixture の実サイズと違う"
    );

    // (module ...) は関数にならないので、生成関数は decl 数 - 1。
    assert_eq!(
        total_functions,
        last_decls - 1,
        "BOOT-04 large-ftable vector-push stage1 build compile progress: 生成関数の総数が decl 数 - 1 でない"
    );
    // pair ループは decl 1 個あたり 10 値、前後に 12 + 6 値が付く。
    assert_eq!(
        values.len() as i64,
        18 + 10 * (last_decls - 1),
        "BOOT-04 large-ftable vector-push stage1 build compile progress: progress の長さが decl 数から決まる形になっていない"
    );
    // fixture は test 内のリテラル / 生成ループから決まるので decl 数は exact に固定できる。
    assert_eq!(last_decls, 1016, "BOOT-04 large-ftable vector-push stage1 build compile progress: fixture の decl 数が変わった (実測 1016 / 2026-08-27)");
}
