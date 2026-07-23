use std::path::{Path, PathBuf};

use lsharp_ir::Module;

/// compile バックエンドターゲット
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileTarget {
    WasiPreview1,
    WasiComponent,
    WebWasm,
    Native,
}

/// compile の値表現 backend。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileBackend {
    /// 現行の linear-memory / target 別 backend。
    Linear,
    /// WasmGC struct/array を使う optional backend。
    WasmGc,
}

/// compile パイプラインの結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileArtifacts {
    pub output_path: PathBuf,
    pub formatted: bool,
}

/// 明示指定または出力拡張子から compile target を決定する
pub fn resolve_compile_target(
    output: Option<&Path>,
    requested_target: Option<CompileTarget>,
) -> miette::Result<(CompileTarget, PathBuf)> {
    let output_path = output
        .map(Path::to_path_buf)
        .ok_or_else(|| miette::miette!("output path が必要です"))?;
    let target = requested_target.unwrap_or_else(|| infer_target_from_output_path(&output_path));
    Ok((target, output_path))
}

fn infer_target_from_output_path(output_path: &Path) -> CompileTarget {
    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if output_name.ends_with(".component.wasm") {
        CompileTarget::WasiComponent
    } else {
        match output_path.extension().and_then(|ext| ext.to_str()) {
            Some("wasm") => CompileTarget::WasiPreview1,
            _ => CompileTarget::Native,
        }
    }
}

fn default_output_path(file: &Path, target: CompileTarget) -> PathBuf {
    match target {
        CompileTarget::WasiPreview1 => file.with_extension("wasm"),
        CompileTarget::WasiComponent => file.with_extension("component.wasm"),
        CompileTarget::WebWasm => file.with_extension("wasm"),
        CompileTarget::Native => {
            let stem = file
                .file_stem()
                .map(|stem| stem.to_os_string())
                .or_else(|| file.file_name().map(|name| name.to_os_string()))
                .unwrap_or_else(|| "a.out".into());
            file.with_file_name(stem)
        }
    }
}

/// コンパイル前にフォーマットを適用し、必要ならソースを書き戻す
pub fn prepare_source_for_compile(file: &Path) -> miette::Result<(String, bool)> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let formatted =
        crate::fmt::format_source(&source).map_err(|e| miette::miette!("フォーマット失敗: {e}"))?;

    if formatted != source {
        std::fs::write(file, &formatted)
            .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
        return Ok((formatted, true));
    }

    Ok((source, false))
}

fn compile_module_from_formatted_source(
    file: &Path,
    source: &str,
    backend: CompileBackend,
) -> miette::Result<Module> {
    if has_file_imports_from_source(source) {
        if backend == CompileBackend::WasmGc {
            return Err(miette::miette!(
                "[LS4001] WasmGC backend は現時点で import を含む compile をサポートしていません"
            ));
        }
        return lsharp_ir::compile_multi_file(file).map_err(|e| miette::miette!("{e}"));
    }

    let program =
        lsharp_syntax::parse(source).map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
    let expr_type_results = infer.expr_type_results_snapshot();
    let lower_backend = match backend {
        CompileBackend::Linear => lsharp_ir::lower::LowerBackend::Linear,
        CompileBackend::WasmGc => lsharp_ir::lower::LowerBackend::WasmGc,
    };
    let mut lower = lsharp_ir::lower::Lower::with_backend(lower_backend);
    lower
        .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
        .map_err(|e| miette::miette!("[{}] {e}", e.code()))
}

fn has_file_imports_from_source(source: &str) -> bool {
    match lsharp_syntax::parse(source) {
        Ok(program) => program
            .decls
            .iter()
            .any(|decl| matches!(decl, lsharp_syntax::ast::Decl::ImportDecl { .. })),
        Err(_) => false,
    }
}

/// format -> check -> codegen の統合 compile パイプライン
pub fn compile_file(
    file: &Path,
    output: Option<&Path>,
    emit_ir: bool,
    requested_target: Option<CompileTarget>,
) -> miette::Result<CompileArtifacts> {
    compile_file_with_backend(
        file,
        output,
        emit_ir,
        requested_target,
        CompileBackend::Linear,
    )
}

/// backend を明示した format -> check -> codegen の統合 compile パイプライン。
pub fn compile_file_with_backend(
    file: &Path,
    output: Option<&Path>,
    emit_ir: bool,
    requested_target: Option<CompileTarget>,
    backend: CompileBackend,
) -> miette::Result<CompileArtifacts> {
    let (target, output_path) = if let Some(output_path) = output {
        resolve_compile_target(Some(output_path), requested_target)?
    } else {
        let target = requested_target.unwrap_or(match backend {
            CompileBackend::Linear => CompileTarget::WasiComponent,
            CompileBackend::WasmGc => CompileTarget::WebWasm,
        });
        let output_path = default_output_path(file, target);
        (target, output_path)
    };
    let (formatted_source, formatted) = prepare_source_for_compile(file)?;
    let module = compile_module_from_formatted_source(file, &formatted_source, backend)?;

    if emit_ir {
        print!("{}", module.dump());
        return Ok(CompileArtifacts {
            output_path,
            formatted,
        });
    }

    match backend {
        CompileBackend::Linear => match target {
            CompileTarget::WasiPreview1 => {
                let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
                    .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
                std::fs::write(&output_path, &wasm_bytes)
                    .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
            }
            CompileTarget::WasiComponent => {
                let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi_p2(&module)
                    .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
                std::fs::write(&output_path, &wasm_bytes)
                    .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
            }
            CompileTarget::WebWasm => {
                let wasm_bytes = lsharp_wasm::codegen::emit_wasm(&module)
                    .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
                std::fs::write(&output_path, &wasm_bytes)
                    .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
            }
            CompileTarget::Native => {
                crate::native::compile_native_executable(&module, &output_path)?;
            }
        },
        CompileBackend::WasmGc => {
            if target != CompileTarget::WebWasm {
                return Err(miette::miette!(
                    "[LS4001] WasmGC backend は現在 --target web-wasm と組み合わせてください"
                ));
            }
            let wasm_bytes = lsharp_wasm::wasmgc::emit_wasm_wasmgc(&module)
                .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
            std::fs::write(&output_path, &wasm_bytes)
                .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
        }
    }
    Ok(CompileArtifacts {
        output_path,
        formatted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_resolve_compile_target_uses_output_extension_when_flag_missing() {
        let component_output = Path::new("demo.component.wasm");
        let wasm_output = Path::new("demo.wasm");
        let native_output = Path::new("demo");

        let (component_target, component_path) =
            resolve_compile_target(Some(component_output), None).unwrap();
        let (wasm_target, wasm_path) = resolve_compile_target(Some(wasm_output), None).unwrap();
        let (native_target, native_path) =
            resolve_compile_target(Some(native_output), None).unwrap();

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
    fn test_wasmgc_backend_rejects_unsupported_record_string_literal_pattern() {
        let error = compile_module_from_formatted_source(
            Path::new("Main.ls"),
            "(type Point (record (: x String)))\n\
             (type Box (Box Point))\n\
             (defn read-point [value]\n\
               (match value [(Box {Point x \"value\"}) 1] [_ 0]))\n",
            CompileBackend::WasmGc,
        )
        .expect_err("WasmGC backend は未対応の record literal pattern を暗黙に linear lowering してはならない");
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
        let stdout = lsharp_wasm::wasi_runner::run_wasm_component(&wasm_bytes).expect(
            "wasi-component target は preview2 runner で実行できる component を出力するべき",
        );
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
        let status = std::process::Command::new(&artifacts.output_path)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(42));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_compile_file_native_target_ignores_unreachable_runtime_helpers_aarch64_macos() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_reachable");
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
        let status = std::process::Command::new(&artifacts.output_path)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(42));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_compile_file_native_target_executes_simple_i64_arithmetic_aarch64_macos() {
        let dir =
            std::env::temp_dir().join("lsharp_compile_pipeline_native_aarch64_macos_arithmetic");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(dir.join(".git")).unwrap();

        let file = dir.join("Main.ls");
        let output = dir.join("demo");
        std::fs::write(&file, "(defn main [] (+ 40 2))\n").unwrap();

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
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

        let artifacts =
            compile_file(&file, Some(&output), false, Some(CompileTarget::Native)).unwrap();
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
            message.contains("native backend は未サポート"),
            "native target の明示エラーが必要: {message}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
