
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
    let values = parse_progress_values(&progress_output, "BOOT-04 minimal path-parent build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 minimal path-parent build compile progress");

    // import を持たない単一 fixture なので、先頭 pair と末尾 pair は同じものを指す。
    assert_eq!(values[2], 1, "BOOT-04 minimal path-parent build compile progress: import 無しなので pair 数は 1");
    assert_eq!(
        first_bytes, last_bytes,
        "BOOT-04 minimal path-parent build compile progress: 先頭と末尾の src バイト数が一致しない"
    );
    assert_eq!(
        first_decls, last_decls,
        "BOOT-04 minimal path-parent build compile progress: 先頭と末尾の decl 数が一致しない"
    );

    // fixture は本 test 自身が書いたものなので、compiler が読んだバイト数は実サイズと一致するはず。
    // 食い違うなら compiler が読んだのは別のファイルである。
    let written_bytes = std::fs::metadata(&source_path)
        .expect("BOOT-04 minimal path-parent build compile progress: fixture の metadata が取れない")
        .len() as i64;
    assert_eq!(
        last_bytes, written_bytes,
        "BOOT-04 minimal path-parent build compile progress: compiler が読んだバイト数が fixture の実サイズと違う"
    );

    // (module ...) は関数にならないので、生成関数は decl 数 - 1。
    assert_eq!(
        total_functions,
        last_decls - 1,
        "BOOT-04 minimal path-parent build compile progress: 生成関数の総数が decl 数 - 1 でない"
    );
    // pair ループは decl 1 個あたり 10 値、前後に 12 + 6 値が付く。
    assert_eq!(
        values.len() as i64,
        18 + 10 * (last_decls - 1),
        "BOOT-04 minimal path-parent build compile progress: progress の長さが decl 数から決まる形になっていない"
    );
    // fixture は test 内のリテラル / 生成ループから決まるので decl 数は exact に固定できる。
    assert_eq!(last_decls, 7, "BOOT-04 minimal path-parent build compile progress: fixture の decl 数が変わった (実測 7 / 2026-08-27)");
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
    let values = parse_progress_values(&progress_output, "BOOT-04 App/Cli stage1 build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 App/Cli stage1 build compile progress");

    // import 閉包に含まれる module 数。import 文を足し引きしたときだけ動くので exact に固定する。
    assert_eq!(
        values[2], 44,
        "BOOT-04 App/Cli stage1 build compile progress: import 閉包の module 数が変わった (実測 44 / 2026-08-27)"
    );
    // import した module の関数も乗るので、生成関数は末尾 module の decl 数より必ず多い。
    assert!(
        total_functions > last_decls,
        "BOOT-04 App/Cli stage1 build compile progress: 生成関数 {total_functions} が末尾 module の decl 数 {last_decls} 以下"
    );
    // 以下は `.ls` を 1 行編集するだけで動くので下限だけを見る。
    // 実測 (2026-08-27): 先頭 12043B/39 decl, 末尾 133977B/403 decl, 生成関数 4605。
    assert!(
        first_bytes > 6021 && first_decls > 19,
        "BOOT-04 App/Cli stage1 build compile progress: 先頭 module が小さすぎる: {first_bytes}B/{first_decls} decl"
    );
    assert!(
        last_bytes > 66988 && last_decls > 201,
        "BOOT-04 App/Cli stage1 build compile progress: 末尾 module が小さすぎる: {last_bytes}B/{last_decls} decl"
    );
    assert!(
        total_functions > 2302,
        "BOOT-04 App/Cli stage1 build compile progress: 生成関数の総数が少なすぎる: {total_functions}"
    );
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
    let values = parse_progress_values(&progress_output, "BOOT-04 Compiler.ls stage1 build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 Compiler.ls stage1 build compile progress");

    // import 閉包に含まれる module 数。import 文を足し引きしたときだけ動くので exact に固定する。
    assert_eq!(
        values[2], 6,
        "BOOT-04 Compiler.ls stage1 build compile progress: import 閉包の module 数が変わった (実測 6 / 2026-08-27)"
    );
    // import した module の関数も乗るので、生成関数は末尾 module の decl 数より必ず多い。
    assert!(
        total_functions > last_decls,
        "BOOT-04 Compiler.ls stage1 build compile progress: 生成関数 {total_functions} が末尾 module の decl 数 {last_decls} 以下"
    );
    // 以下は `.ls` を 1 行編集するだけで動くので下限だけを見る。
    // 実測 (2026-08-27): 先頭 1916B/45 decl, 末尾 210442B/312 decl, 生成関数 847。
    assert!(
        first_bytes > 958 && first_decls > 22,
        "BOOT-04 Compiler.ls stage1 build compile progress: 先頭 module が小さすぎる: {first_bytes}B/{first_decls} decl"
    );
    assert!(
        last_bytes > 105221 && last_decls > 156,
        "BOOT-04 Compiler.ls stage1 build compile progress: 末尾 module が小さすぎる: {last_bytes}B/{last_decls} decl"
    );
    assert!(
        total_functions > 423,
        "BOOT-04 Compiler.ls stage1 build compile progress: 生成関数の総数が少なすぎる: {total_functions}"
    );
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
    let values = parse_progress_values(&progress_output, "BOOT-04 CompilerMode.ls stage1 build compile progress");
    let (first_bytes, first_decls, last_bytes, last_decls, total_functions) =
        assert_build_compile_progress_shape(&values, "BOOT-04 CompilerMode.ls stage1 build compile progress");

    // import 閉包に含まれる module 数。import 文を足し引きしたときだけ動くので exact に固定する。
    assert_eq!(
        values[2], 12,
        "BOOT-04 CompilerMode.ls stage1 build compile progress: import 閉包の module 数が変わった (実測 12 / 2026-08-27)"
    );
    // import した module の関数も乗るので、生成関数は末尾 module の decl 数より必ず多い。
    assert!(
        total_functions > last_decls,
        "BOOT-04 CompilerMode.ls stage1 build compile progress: 生成関数 {total_functions} が末尾 module の decl 数 {last_decls} 以下"
    );
    // 以下は `.ls` を 1 行編集するだけで動くので下限だけを見る。
    // 実測 (2026-08-27): 先頭 12043B/39 decl, 末尾 290763B/318 decl, 生成関数 1942。
    assert!(
        first_bytes > 6021 && first_decls > 19,
        "BOOT-04 CompilerMode.ls stage1 build compile progress: 先頭 module が小さすぎる: {first_bytes}B/{first_decls} decl"
    );
    assert!(
        last_bytes > 145381 && last_decls > 159,
        "BOOT-04 CompilerMode.ls stage1 build compile progress: 末尾 module が小さすぎる: {last_bytes}B/{last_decls} decl"
    );
    assert!(
        total_functions > 971,
        "BOOT-04 CompilerMode.ls stage1 build compile progress: 生成関数の総数が少なすぎる: {total_functions}"
    );
}
