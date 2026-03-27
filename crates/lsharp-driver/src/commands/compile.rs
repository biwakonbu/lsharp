use std::path::{Path, PathBuf};

use lsharp_ir::Module;

/// compile パイプラインの結果
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileArtifacts {
    pub output_path: PathBuf,
    pub formatted: bool,
}

/// コンパイル前にフォーマットを適用し、必要ならソースを書き戻す
pub fn prepare_source_for_compile(file: &Path) -> miette::Result<(String, bool)> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let formatted = super::fmt::format_source(&source)
        .map_err(|e| miette::miette!("フォーマット失敗: {e}"))?;

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

    let program = lsharp_syntax::parse(source)
        .map_err(|e| miette::miette!("{e}"))?;
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
) -> miette::Result<CompileArtifacts> {
    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| file.with_extension("wasm"));
    let (formatted_source, formatted) = prepare_source_for_compile(file)?;
    let module = compile_module_from_formatted_source(file, &formatted_source)?;

    if emit_ir {
        print!("{}", module.dump());
        return Ok(CompileArtifacts {
            output_path,
            formatted,
        });
    }

    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| miette::miette!("{e}"))?;
    std::fs::write(&output_path, &wasm_bytes)
        .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
    Ok(CompileArtifacts {
        output_path,
        formatted,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(formatted, on_disk, "compile 前にフォーマット済みソースを書き戻すべき");
        assert!(
            on_disk.contains("(defn main"),
            "compile 前に空白が正規化されるべき: {on_disk}"
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
        let output = dir.join("Main.wasm");
        std::fs::write(&file, "(defn   main  []   42)\n").unwrap();

        let artifacts = compile_file(&file, Some(&output), false).unwrap();

        assert_eq!(artifacts.output_path, output);
        assert!(artifacts.formatted, "compile は format 差分を検出して書き戻すべき");
        assert!(output.exists(), "compile は Wasm 出力を生成するべき");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
