use std::path::{Path, PathBuf};

use lsharp_ir::Module;

/// compile バックエンドターゲット
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileTarget {
    WasiComponent,
    WebWasm,
    Native,
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
    let output_name = output_path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    if output_name.ends_with(".component.wasm") {
        CompileTarget::WasiComponent
    } else {
        match output_path.extension().and_then(|ext| ext.to_str()) {
            Some("wasm") => CompileTarget::WasiComponent,
            _ => CompileTarget::Native,
        }
    }
}

fn default_output_path(file: &Path, target: CompileTarget) -> PathBuf {
    match target {
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
        super::fmt::format_source(&source).map_err(|e| miette::miette!("フォーマット失敗: {e}"))?;

    if formatted != source {
        std::fs::write(file, &formatted)
            .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
        return Ok((formatted, true));
    }

    Ok((source, false))
}

fn compile_module_from_formatted_source(file: &Path, source: &str) -> miette::Result<Module> {
    if has_file_imports_from_source(source) {
        return lsharp_ir::compile_multi_file(file).map_err(|e| miette::miette!("{e}"));
    }

    let program = lsharp_syntax::parse(source).map_err(|e| miette::miette!("{e}"))?;
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| miette::miette!("{e}"))?;
    let mut lower = lsharp_ir::lower::Lower::new();
    lower
        .lower_program(&program, &type_results)
        .map_err(|e| miette::miette!("{e}"))
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
    let (target, output_path) = if let Some(output_path) = output {
        resolve_compile_target(Some(output_path), requested_target)?
    } else {
        let target = requested_target.unwrap_or(CompileTarget::WasiComponent);
        let output_path = default_output_path(file, target);
        (target, output_path)
    };
    let (formatted_source, formatted) = prepare_source_for_compile(file)?;
    let module = compile_module_from_formatted_source(file, &formatted_source)?;

    if emit_ir {
        print!("{}", module.dump());
        return Ok(CompileArtifacts {
            output_path,
            formatted,
        });
    }

    match target {
        CompileTarget::WasiComponent => {
            let wasm_bytes =
                lsharp_wasm::wasi::emit_wasm_wasi_p2(&module).map_err(|e| miette::miette!("{e}"))?;
            std::fs::write(&output_path, &wasm_bytes)
                .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
        }
        CompileTarget::WebWasm => {
            let wasm_bytes =
                lsharp_wasm::codegen::emit_wasm(&module).map_err(|e| miette::miette!("{e}"))?;
            std::fs::write(&output_path, &wasm_bytes)
                .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
        }
        CompileTarget::Native => {
            return Err(miette::miette!(
                "native backend は未サポートです。現在の Rust driver は wasm のみ生成できます: {}",
                output_path.display()
            ));
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
        assert_eq!(wasm_target, CompileTarget::WasiComponent);
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

        let artifacts = compile_file(&file, Some(&output), false, Some(CompileTarget::WebWasm))
            .unwrap();
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
