use std::path::{Path, PathBuf};

use crate::artifact_cache::ArtifactCache;
use crate::diagnostics::driver_io_error;
use lsharp_ir::{CompilationCache, Module, SourceFingerprint};
use lsharp_wasm::validation::WasmValidationMode;

/// compile バックエンドターゲット
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompileTarget {
    WasiPreview1,
    WasiComponent,
    WebWasm,
    Native,
}

/// compile の値表現 backend。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// artifact cache hit で pipeline を短絡したか。
    pub from_cache: bool,
}

/// process 間 artifact cache が再利用対象を識別するための deterministic key。
///
/// module graph の全 source fingerprint、canonical path、compiler package version、target/backend を
/// 含める。artifact persistence はこの key を使う後続 sliceで接続し、key schemaを変更した場合は
/// `COMPILE_CACHE_KEY_SCHEMA` を更新する。
pub const COMPILE_CACHE_KEY_SCHEMA: &str = "lsharp-compile-key-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompileCacheKey {
    graph_fingerprint: SourceFingerprint,
    target: CompileTarget,
    backend: CompileBackend,
}

impl CompileCacheKey {
    /// entry file から解決された全 module source を読み、compile identityを作る。
    pub fn from_entry(
        entry_file: &Path,
        target: CompileTarget,
        backend: CompileBackend,
    ) -> miette::Result<Self> {
        let (_, mut sorted_files) =
            lsharp_ir::module_graph::ModuleGraph::build_from_entry_with_scc(entry_file).map_err(
                |error| miette::miette!("[{}] モジュールグラフ構築エラー: {error}", error.code()),
            )?;
        sorted_files.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

        let entry_identity = std::fs::canonicalize(entry_file)
            .unwrap_or_else(|_| entry_file.to_path_buf())
            .display()
            .to_string();
        let mut manifest = String::new();
        manifest.push_str(COMPILE_CACHE_KEY_SCHEMA);
        manifest.push('\n');
        manifest.push_str(env!("CARGO_PKG_VERSION"));
        manifest.push('\n');
        manifest.push_str(&entry_identity);
        manifest.push('\n');
        manifest.push_str(target_tag(target));
        manifest.push('\n');
        manifest.push_str(backend_tag(backend));
        manifest.push('\n');

        for (module_name, module_path) in sorted_files {
            let source = std::fs::read_to_string(&module_path)
                .map_err(|error| driver_io_error(format!("{}: {error}", module_path.display())))?;
            let canonical_path = std::fs::canonicalize(&module_path)
                .unwrap_or(module_path)
                .display()
                .to_string();
            manifest.push_str(&module_name);
            manifest.push('\0');
            manifest.push_str(&canonical_path);
            manifest.push('\0');
            manifest.push_str(&SourceFingerprint::from_source(&source).to_string());
            manifest.push('\n');
        }

        Ok(Self {
            graph_fingerprint: SourceFingerprint::from_source(&manifest),
            target,
            backend,
        })
    }

    pub fn fingerprint(&self) -> SourceFingerprint {
        self.graph_fingerprint
    }

    pub fn target(&self) -> CompileTarget {
        self.target
    }

    pub fn backend(&self) -> CompileBackend {
        self.backend
    }
}

fn target_tag(target: CompileTarget) -> &'static str {
    match target {
        CompileTarget::WasiPreview1 => "wasi-preview1",
        CompileTarget::WasiComponent => "wasi-component",
        CompileTarget::WebWasm => "web-wasm",
        CompileTarget::Native => "native",
    }
}

fn backend_tag(backend: CompileBackend) -> &'static str {
    match backend {
        CompileBackend::Linear => "linear",
        CompileBackend::WasmGc => "wasmgc",
    }
}

/// 同一 process / host session 内で compile cache を保持する境界。
///
/// cache は entry root の切り替え時に `CompilationCache` が scope を分離する。process 間 artifact
/// persistence は `with_artifact_cache` で明示的に有効化した場合だけ行う。
#[derive(Debug, Default)]
pub struct CompileSession {
    cache: CompilationCache,
    artifact_cache: Option<ArtifactCache>,
}

impl CompileSession {
    pub fn new() -> Self {
        Self::default()
    }

    /// 明示 root の process 間 artifact cache を使う compile session を作る。
    pub fn with_artifact_cache(root: impl Into<PathBuf>) -> Self {
        Self {
            cache: CompilationCache::new(),
            artifact_cache: Some(ArtifactCache::new(root)),
        }
    }

    /// session が保持している module cache entry 数を返す。
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }

    /// session cache を使って compile する。
    pub fn compile_file_with_backend(
        &mut self,
        file: &Path,
        output: Option<&Path>,
        emit_ir: bool,
        requested_target: Option<CompileTarget>,
        backend: CompileBackend,
    ) -> miette::Result<CompileArtifacts> {
        compile_file_with_backend_and_cache_internal(
            file,
            output,
            emit_ir,
            requested_target,
            backend,
            &mut self.cache,
            self.artifact_cache.as_ref(),
        )
    }
}

/// 明示指定または出力拡張子から compile target を決定する
pub fn resolve_compile_target(
    output: Option<&Path>,
    requested_target: Option<CompileTarget>,
) -> miette::Result<(CompileTarget, PathBuf)> {
    let output_path = output
        .map(Path::to_path_buf)
        .ok_or_else(|| driver_io_error("output path が必要です"))?;
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
    let source = std::fs::read_to_string(file)
        .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;
    let formatted =
        crate::fmt::format_source(&source).map_err(|e| miette::miette!("フォーマット失敗: {e}"))?;

    if formatted != source {
        std::fs::write(file, &formatted)
            .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;
        return Ok((formatted, true));
    }

    Ok((source, false))
}

#[cfg(test)]
fn compile_module_from_formatted_source(
    file: &Path,
    source: &str,
    backend: CompileBackend,
) -> miette::Result<Module> {
    let mut cache = CompilationCache::new();
    compile_module_from_formatted_source_with_cache(file, source, backend, &mut cache)
}

fn compile_module_from_formatted_source_with_cache(
    file: &Path,
    source: &str,
    backend: CompileBackend,
    cache: &mut CompilationCache,
) -> miette::Result<Module> {
    if has_file_imports_from_source(source) {
        if backend == CompileBackend::WasmGc {
            return Err(miette::miette!(
                "[LS4001] WasmGC backend は現時点で import を含む compile をサポートしていません"
            ));
        }
        return lsharp_ir::compile_multi_file_with_cache(file, cache)
            .map_err(|e| miette::miette!("{e}"));
    }

    let program =
        lsharp_syntax::parse(source).map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
    // block 形式 module body は infer より前に弾く (I-39)
    lsharp_ir::reject_block_form_module_body(&program)
        .map_err(|e| miette::miette!("{}: {e}", file.display()))?;
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

fn write_compile_artifact(path: &Path, bytes: &[u8]) -> miette::Result<()> {
    lsharp_wasm::component_adapter::write_wasm_artifact(path, bytes)
        .map_err(|error| driver_io_error(format!("{}: {}", path.display(), error)))
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
    let mut session = CompileSession::new();
    session.compile_file_with_backend(file, output, emit_ir, requested_target, backend)
}

/// backend を明示し、呼び出し元が保持する解析/IR cache を使う compile パイプライン。
///
/// 通常の `compile_file_with_backend` は互換性のため一時 cache を生成する。LSP や host
/// session のように同一 process で複数回 compile する caller はこの入口へ cache を渡す。
pub fn compile_file_with_backend_and_cache(
    file: &Path,
    output: Option<&Path>,
    emit_ir: bool,
    requested_target: Option<CompileTarget>,
    backend: CompileBackend,
    cache: &mut CompilationCache,
) -> miette::Result<CompileArtifacts> {
    compile_file_with_backend_and_cache_internal(
        file,
        output,
        emit_ir,
        requested_target,
        backend,
        cache,
        None,
    )
}

fn compile_file_with_backend_and_cache_internal(
    file: &Path,
    output: Option<&Path>,
    emit_ir: bool,
    requested_target: Option<CompileTarget>,
    backend: CompileBackend,
    cache: &mut CompilationCache,
    artifact_cache: Option<&ArtifactCache>,
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
    let artifact_key = if artifact_cache.is_some() && !emit_ir && target != CompileTarget::Native {
        Some(CompileCacheKey::from_entry(file, target, backend)?)
    } else {
        None
    };
    if let (Some(artifact_cache), Some(artifact_key)) = (artifact_cache, artifact_key.as_ref())
        && let Some(bytes) = artifact_cache.load(artifact_key)?
        && validate_cached_artifact(artifact_key, &bytes)
    {
        write_compile_artifact(&output_path, &bytes)?;
        return Ok(CompileArtifacts {
            output_path,
            formatted,
            from_cache: true,
        });
    }
    let module =
        compile_module_from_formatted_source_with_cache(file, &formatted_source, backend, cache)?;

    if emit_ir {
        print!("{}", module.dump());
        return Ok(CompileArtifacts {
            output_path,
            formatted,
            from_cache: false,
        });
    }

    match backend {
        CompileBackend::Linear => match target {
            CompileTarget::WasiPreview1 => {
                let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
                    .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
                write_compile_artifact_and_cache(
                    &output_path,
                    &wasm_bytes,
                    artifact_cache,
                    artifact_key.as_ref(),
                )?;
            }
            CompileTarget::WasiComponent => {
                let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi_p2(&module)
                    .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
                write_compile_artifact_and_cache(
                    &output_path,
                    &wasm_bytes,
                    artifact_cache,
                    artifact_key.as_ref(),
                )?;
            }
            CompileTarget::WebWasm => {
                let wasm_bytes = lsharp_wasm::codegen::emit_wasm(&module)
                    .map_err(|e| miette::miette!("[{}] {e}", e.code()))?;
                write_compile_artifact_and_cache(
                    &output_path,
                    &wasm_bytes,
                    artifact_cache,
                    artifact_key.as_ref(),
                )?;
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
            write_compile_artifact_and_cache(
                &output_path,
                &wasm_bytes,
                artifact_cache,
                artifact_key.as_ref(),
            )?;
        }
    }
    Ok(CompileArtifacts {
        output_path,
        formatted,
        from_cache: false,
    })
}

fn validate_cached_artifact(key: &CompileCacheKey, bytes: &[u8]) -> bool {
    let mode = match (key.target(), key.backend()) {
        (CompileTarget::WasiComponent, CompileBackend::Linear) => WasmValidationMode::Component,
        (CompileTarget::WasiPreview1, CompileBackend::Linear)
        | (CompileTarget::WebWasm, CompileBackend::Linear) => WasmValidationMode::Core,
        (CompileTarget::WebWasm, CompileBackend::WasmGc) => WasmValidationMode::CoreWasmGc,
        (CompileTarget::Native, _) | (_, CompileBackend::WasmGc) => return false,
    };
    lsharp_wasm::validation::validate_wasm_artifact(bytes, mode).is_ok()
}

fn write_compile_artifact_and_cache(
    output_path: &Path,
    bytes: &[u8],
    artifact_cache: Option<&ArtifactCache>,
    artifact_key: Option<&CompileCacheKey>,
) -> miette::Result<()> {
    if let (Some(_artifact_cache), Some(artifact_key)) = (artifact_cache, artifact_key)
        && !validate_cached_artifact(artifact_key, bytes)
    {
        return Err(miette::miette!(
            "生成された Wasm artifact が target/backend の検証に失敗しました"
        ));
    }
    write_compile_artifact(output_path, bytes)?;
    if let (Some(artifact_cache), Some(artifact_key)) = (artifact_cache, artifact_key) {
        artifact_cache.store(artifact_key, bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    include!("compile_tests_cache.rs");
    include!("compile_tests_diagnostics.rs");
    include!("compile_tests_wasmgc_a.rs");
    include!("compile_tests_wasmgc_b.rs");
    include!("compile_tests_outputs.rs");
}
