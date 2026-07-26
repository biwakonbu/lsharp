#[test]
fn test_compile_target_and_backend_tags_are_stable() {
    assert_eq!(target_tag(CompileTarget::WasiComponent), "wasi-component");
    assert_eq!(target_tag(CompileTarget::Native), "native");
    assert_eq!(backend_tag(CompileBackend::Linear), "linear");
    assert_eq!(backend_tag(CompileBackend::WasmGc), "wasmgc");
}

#[test]
fn test_compile_file_with_backend_and_cache_reuses_multi_file_cache() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_with_cache_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let main_path = dir.join("Main.ls");
    let output_path = dir.join("Main.wasm");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &main_path,
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
    )
    .unwrap();

    let mut cache = lsharp_ir::CompilationCache::new();
    let first = compile_file_with_backend_and_cache(
        &main_path,
        Some(&output_path),
        false,
        Some(CompileTarget::WasiPreview1),
        CompileBackend::Linear,
        &mut cache,
    )
    .unwrap();
    let first_bytes = std::fs::read(&first.output_path).unwrap();
    assert_eq!(
        cache.len(),
        2,
        "tooling compile は multi-file cache を埋めるべき"
    );

    let second = compile_file_with_backend_and_cache(
        &main_path,
        Some(&output_path),
        false,
        Some(CompileTarget::WasiPreview1),
        CompileBackend::Linear,
        &mut cache,
    )
    .unwrap();
    assert_eq!(first.output_path, second.output_path);
    assert_eq!(first_bytes, std::fs::read(&second.output_path).unwrap());
    assert_eq!(
        cache.len(),
        2,
        "warm tooling compile は cache scope を維持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_cache_key_changes_when_imported_source_changes() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_key_import_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let main_path = dir.join("Main.ls");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &main_path,
        "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();

    let first = CompileCacheKey::from_entry(
        &main_path,
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 8)\n").unwrap();
    let second = CompileCacheKey::from_entry(
        &main_path,
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();

    assert_ne!(
        first, second,
        "imported source の変更は compile key を変えるべき"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_cache_key_changes_when_import_graph_changes() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_key_graph_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let alt_path = dir.join("Alt.ls");
    let main_path = dir.join("Main.ls");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(&alt_path, "(module Alt)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &main_path,
        "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();

    let first = CompileCacheKey::from_entry(
        &main_path,
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    std::fs::write(
        &main_path,
        "(module Main)\n(import Alt)\n(defn main [] (helper))\n",
    )
    .unwrap();
    let second = CompileCacheKey::from_entry(
        &main_path,
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();

    assert_ne!(
        first, second,
        "依存 SCC を含む module graph の変更は artifact compile key を変えるべき"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_cache_key_includes_target_and_backend() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_key_target_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let main_path = dir.join("Main.ls");
    std::fs::write(&main_path, "(module Main)\n(defn main [] 7)\n").unwrap();

    let linear = CompileCacheKey::from_entry(
        &main_path,
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    let component = CompileCacheKey::from_entry(
        &main_path,
        CompileTarget::WasiComponent,
        CompileBackend::Linear,
    )
    .unwrap();
    let wasmgc =
        CompileCacheKey::from_entry(&main_path, CompileTarget::WebWasm, CompileBackend::WasmGc)
            .unwrap();

    assert_ne!(
        linear, component,
        "output target は compile key に含めるべき"
    );
    assert_ne!(linear, wasmgc, "backend は compile key に含めるべき");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_session_reuses_default_cache_for_multi_file_compile() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_session_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let main_path = dir.join("Main.ls");
    let output_path = dir.join("Main.wasm");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &main_path,
        "(module Main)\n(import Lib)\n(defn main [] (+ (helper) 1))\n",
    )
    .unwrap();

    let mut session = CompileSession::new();
    let first = session
        .compile_file_with_backend(
            &main_path,
            Some(&output_path),
            false,
            Some(CompileTarget::WasiPreview1),
            CompileBackend::Linear,
        )
        .unwrap();
    let first_bytes = std::fs::read(&first.output_path).unwrap();
    assert_eq!(
        session.cache_len(),
        2,
        "session は multi-file cache を保持するべき"
    );

    let second = session
        .compile_file_with_backend(
            &main_path,
            Some(&output_path),
            false,
            Some(CompileTarget::WasiPreview1),
            CompileBackend::Linear,
        )
        .unwrap();
    assert_eq!(first.output_path, second.output_path);
    assert_eq!(first_bytes, std::fs::read(&second.output_path).unwrap());
    assert_eq!(
        session.cache_len(),
        2,
        "warm session compile は cache scope を維持するべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_session_opt_in_artifact_cache_reuses_across_sessions() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_session_artifact_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    let cache_root = dir.join("artifact-cache");
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let source_path = dir.join("Main.ls");
    let first_output = dir.join("first.wasm");
    let second_output = dir.join("second.wasm");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &source_path,
        "(module Main)\n(import Lib)\n(defn main [] (print (helper)))\n",
    )
    .unwrap();

    let mut cold = CompileSession::with_artifact_cache(&cache_root);
    let first = cold
        .compile_file_with_backend(
            &source_path,
            Some(&first_output),
            false,
            Some(CompileTarget::WasiPreview1),
            CompileBackend::Linear,
        )
        .unwrap();
    let first_bytes = std::fs::read(&first.output_path).unwrap();
    assert!(
        !first.from_cache,
        "cold compile は cache miss として記録するべき"
    );
    assert_eq!(
        cold.cache_len(),
        2,
        "cold compile は IR cache を構築するべき"
    );

    let mut warm = CompileSession::with_artifact_cache(&cache_root);
    let second = warm
        .compile_file_with_backend(
            &source_path,
            Some(&second_output),
            false,
            Some(CompileTarget::WasiPreview1),
            CompileBackend::Linear,
        )
        .unwrap();
    assert_eq!(first_bytes, std::fs::read(&second.output_path).unwrap());
    let second_runtime_output =
        lsharp_wasm::wasi_runner::run_wasm_wasi(&std::fs::read(&second.output_path).unwrap())
            .expect("artifact cache hit の Wasm は runtime 実行できるべき");
    assert_eq!(
        second_runtime_output, "7\n",
        "cache hit output の runtime semantics は cold compile と一致するべき"
    );
    assert!(
        second.from_cache,
        "cross-session artifact cache hit を observable にするべき"
    );
    assert_eq!(
        warm.cache_len(),
        0,
        "artifact cache hit は同一 process の IR compile を再実行しないべき"
    );

    std::fs::write(
        &source_path,
        "(module Main)\n(import Lib)\n(defn main [] (print (+ (helper) 1)))\n",
    )
    .unwrap();
    let changed_output = dir.join("changed.wasm");
    let mut changed = CompileSession::with_artifact_cache(&cache_root);
    let changed_artifacts = changed
        .compile_file_with_backend(
            &source_path,
            Some(&changed_output),
            false,
            Some(CompileTarget::WasiPreview1),
            CompileBackend::Linear,
        )
        .unwrap();
    assert!(
        !changed_artifacts.from_cache,
        "source fingerprint が変わった compile は cache miss として記録するべき"
    );
    assert_eq!(
        changed.cache_len(),
        2,
        "source fingerprint が変わった場合は fresh compile へ戻るべき"
    );
    let changed_runtime_output = lsharp_wasm::wasi_runner::run_wasm_wasi(
        &std::fs::read(&changed_artifacts.output_path).unwrap(),
    )
    .expect("source change 後の fresh Wasm は runtime 実行できるべき");
    assert_eq!(changed_runtime_output, "8\n");
    assert_ne!(
        first_bytes,
        std::fs::read(&changed_artifacts.output_path).unwrap(),
        "source 変更後に stale artifact を返してはいけない"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_session_artifact_cache_rejects_invalid_wasm_payload() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_session_artifact_invalid_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    let cache_root = dir.join("artifact-cache");
    std::fs::create_dir_all(&dir).unwrap();
    let lib_path = dir.join("Lib.ls");
    let source_path = dir.join("Main.ls");
    let output_path = dir.join("Main.wasm");
    std::fs::write(&lib_path, "(module Lib)\n(defn helper [] 7)\n").unwrap();
    std::fs::write(
        &source_path,
        "(module Main)\n(import Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();

    let key = CompileCacheKey::from_entry(
        &source_path,
        CompileTarget::WasiPreview1,
        CompileBackend::Linear,
    )
    .unwrap();
    ArtifactCache::new(&cache_root)
        .store(&key, b"not-a-wasm")
        .unwrap();

    let mut session = CompileSession::with_artifact_cache(&cache_root);
    session
        .compile_file_with_backend(
            &source_path,
            Some(&output_path),
            false,
            Some(CompileTarget::WasiPreview1),
            CompileBackend::Linear,
        )
        .unwrap();
    let output = std::fs::read(&output_path).unwrap();
    assert_eq!(
        &output[..4],
        b"\0asm",
        "不正な cache payload を output に出してはいけない"
    );
    assert_eq!(
        session.cache_len(),
        2,
        "Wasm validation failure は fresh compile へ戻るべき"
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_compile_artifact_writer_uses_atomic_wasm_boundary() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_compile_atomic_artifact_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("compile artifact directory を作成できる");
    let path = dir.join("Main.wasm");
    write_compile_artifact(&path, b"compiled-wasm")
        .expect("compile artifact を atomic に保存できる");
    assert_eq!(
        lsharp_wasm::component_adapter::read_wasm_artifact(&path)
            .expect("compile artifact を再読込できる"),
        b"compiled-wasm"
    );
    let entries = std::fs::read_dir(&dir)
        .expect("compile artifact directory を列挙できる")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("compile artifact directory entry を取得できる");
    assert_eq!(entries.len(), 1, "compile artifact の一時 file を残さない");
    assert_eq!(entries[0].file_name(), "Main.wasm");
    std::fs::remove_dir_all(&dir).expect("compile artifact directory を削除できる");
}

#[test]
fn compile_diagnostics_preserve_stable_type_error_code() {
    let error = compile_module_from_formatted_source(
        Path::new("Main.ls"),
        "(defn bad [] (+ 1 true))",
        CompileBackend::Linear,
    )
    .expect_err("型エラーは compile を失敗させるべき");

    assert!(
        error.to_string().contains("[LS1004]"),
        "compile diagnostics は stable code を含むべき: {error}"
    );
}

#[test]
fn compile_diagnostics_preserve_module_graph_error_code() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_compile_module_graph_diagnostic_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("module graph diagnostic directory を作成できる");
    let main_path = dir.join("Main.ls");
    std::fs::write(
        &main_path,
        "(module Main)\n(import Missing)\n(defn main [] 0)\n",
    )
    .expect("module graph diagnostic fixture を書き込める");

    let error = compile_module_from_formatted_source(
        &main_path,
        &std::fs::read_to_string(&main_path).unwrap(),
        CompileBackend::Linear,
    )
    .expect_err("missing module は compile を失敗させるべき");

    assert!(
        error.to_string().contains("[LS3102]"),
        "module graph diagnostics は stable code を含むべき: {error}"
    );
    std::fs::remove_dir_all(&dir).expect("module graph diagnostic directory を削除できる");
}
