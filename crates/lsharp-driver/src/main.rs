//! # Default path / compiler path (OPS-05)
//!
//! - 現行: 本バイナリが Rust 実装パイプライン（syntax → types → ir → wasm）を**内蔵**する。
//! - 移行中: 環境変数 `LSHARP_PATH` で selfhost / 外部コンパイラ executable・その配置ディレクトリ・`.wasm` / `.component.wasm` guest artifact を指せる。
//! - 検証: `scripts/ci/default-path-smoke.sh` が `target/debug/lsharp` 単体で embedded default path の `compile` / `build` を含む smoke を通す。

mod api_doc;
mod atomic_write;
mod claude_plugin;
mod commands;
mod config;
mod doc_site;
#[cfg(test)]
mod error;
mod error_codes;
mod lockfile;
mod mcp_server;
mod resolver;
#[cfg(test)]
#[path = "main_tests.rs"]
mod tests;

use clap::{Parser, Subcommand, ValueEnum};
use error_codes::driver_io_error;
use sha2::{Digest, Sha256};
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;

const EMBEDDED_COMPONENT_BYTES: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/embedded-lsharp.component.wasm"));

fn adjacent_component_sidecar_path_for_executable(executable: &Path) -> Option<PathBuf> {
    let parent = executable.parent()?;
    let stem = executable.file_stem()?.to_str()?;
    Some(parent.join(format!("{stem}.component.wasm")))
}

fn resolve_default_component_bytes() -> miette::Result<Cow<'static, [u8]>> {
    let current_exe =
        std::env::current_exe().map_err(|e| miette::miette!("current exe の取得に失敗: {e}"))?;
    let Some(sidecar_path) = adjacent_component_sidecar_path_for_executable(&current_exe) else {
        return Ok(Cow::Borrowed(EMBEDDED_COMPONENT_BYTES));
    };
    if !sidecar_path.is_file() {
        return Ok(Cow::Borrowed(EMBEDDED_COMPONENT_BYTES));
    }

    let bytes = std::fs::read(&sidecar_path).map_err(|e| {
        driver_io_error(format!(
            "adjacent component sidecar の読み込みに失敗しました ({}): {e}",
            sidecar_path.display()
        ))
    })?;
    Ok(Cow::Owned(bytes))
}

#[derive(Parser)]
#[command(name = "lsharp", version, about = "L# コンパイラ")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum CliCompileTarget {
    #[value(name = "wasi-preview1")]
    WasiPreview1,
    #[value(name = "wasi-component", alias("wasm"))]
    WasiComponent,
    #[value(name = "web-wasm")]
    WebWasm,
    Native,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum CliValidationFormat {
    Text,
    Json,
}

impl From<CliCompileTarget> for commands::compile::CompileTarget {
    fn from(value: CliCompileTarget) -> Self {
        match value {
            CliCompileTarget::WasiPreview1 => Self::WasiPreview1,
            CliCompileTarget::WasiComponent => Self::WasiComponent,
            CliCompileTarget::WebWasm => Self::WebWasm,
            CliCompileTarget::Native => Self::Native,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum CliCompileBackend {
    #[value(name = "linear")]
    Linear,
    #[value(name = "wasmgc")]
    WasmGc,
}

impl From<CliCompileBackend> for commands::compile::CompileBackend {
    fn from(value: CliCompileBackend) -> Self {
        match value {
            CliCompileBackend::Linear => Self::Linear,
            CliCompileBackend::WasmGc => Self::WasmGc,
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// プロジェクトを初期化
    Init {
        /// プロジェクト名
        name: String,
    },

    /// ソースファイルを Wasm にコンパイル
    Compile {
        /// 入力ファイル
        file: PathBuf,

        /// 出力ファイル
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// コンパイルターゲット (`wasi-component` / `web-wasm` / `native`, `wasm` は alias)
        #[arg(long, value_enum)]
        target: Option<CliCompileTarget>,

        /// 値表現 backend (`linear` / `wasmgc`)
        #[arg(long, value_enum)]
        backend: Option<CliCompileBackend>,

        /// process 間 Wasm artifact cache の root（未指定時は LSHARP_ARTIFACT_CACHE_DIR を参照）
        #[arg(long)]
        artifact_cache_dir: Option<PathBuf>,

        /// 明示 artifact cache に残す最大 entry 数（未指定時は LSHARP_ARTIFACT_CACHE_MAX_ENTRIES を参照）
        #[arg(long)]
        artifact_cache_max_entries: Option<usize>,

        /// 明示 artifact cache に残す最大 bytes 数（未指定時は LSHARP_ARTIFACT_CACHE_MAX_BYTES を参照）
        #[arg(long)]
        artifact_cache_max_bytes: Option<u64>,

        /// IR を表示する
        #[arg(long)]
        emit_ir: bool,
    },

    /// ソースファイルを Wasm にコンパイル (compile のエイリアス)
    Build {
        /// 入力ファイル
        file: PathBuf,

        /// 出力ファイル
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// コンパイルターゲット (`wasi-component` / `web-wasm` / `native`, `wasm` は alias)
        #[arg(long, value_enum)]
        target: Option<CliCompileTarget>,

        /// 値表現 backend (`linear` / `wasmgc`)
        #[arg(long, value_enum)]
        backend: Option<CliCompileBackend>,

        /// process 間 Wasm artifact cache の root（未指定時は LSHARP_ARTIFACT_CACHE_DIR を参照）
        #[arg(long)]
        artifact_cache_dir: Option<PathBuf>,

        /// 明示 artifact cache に残す最大 entry 数（未指定時は LSHARP_ARTIFACT_CACHE_MAX_ENTRIES を参照）
        #[arg(long)]
        artifact_cache_max_entries: Option<usize>,

        /// 明示 artifact cache に残す最大 bytes 数（未指定時は LSHARP_ARTIFACT_CACHE_MAX_BYTES を参照）
        #[arg(long)]
        artifact_cache_max_bytes: Option<u64>,

        /// IR を表示する
        #[arg(long)]
        emit_ir: bool,
    },

    /// メタデータテストを実行 (:example, :invariant の自動検証)
    Test {
        /// 入力ファイル
        file: PathBuf,
    },

    /// intent/evidence graph manifest を検証
    Validate {
        /// versioned JSON manifest (省略時は lsharp.toml の [validation].manifest)
        file: Option<PathBuf>,

        /// L# source file を intent graph として検証 (contract/evidence 未接続時は unknown)
        #[arg(long, conflicts_with = "file", value_name = "SOURCE")]
        source: Option<PathBuf>,

        /// 出力形式
        #[arg(long, value_enum, default_value = "text")]
        format: CliValidationFormat,

        /// 構築した graph を versioned JSON manifest として出力
        #[arg(long, value_name = "OUTPUT")]
        emit_manifest: Option<PathBuf>,
    },

    /// ドキュメントレビュー (YAML チェックポイント出力)
    Review {
        /// 入力ファイル
        file: PathBuf,
    },

    /// ドキュメントの確認済みマーク
    DocAck {
        /// 確認する関数名
        name: String,

        /// 確認者名
        #[arg(long, default_value = "anonymous")]
        reviewer: String,
    },

    /// ドキュメント検証 (pre-commit hook 用)
    DocCheck {
        /// 入力ファイル
        file: PathBuf,

        /// ドキュメントレビューをスキップ
        #[arg(long)]
        skip_doc_review: bool,

        /// コミットトレイラーを出力 (Doc-Reviewed-By, Doc-Review-Status)
        #[arg(long)]
        emit_trailers: bool,
    },

    /// 依存パッケージをインストール
    Install,

    /// GitHub パッケージ依存を lsharp.toml に追加
    Add {
        /// GitHub URL (`github.com/user/repo` または `https://github.com/user/repo`)
        github_url: String,

        /// 利用する Git tag
        #[arg(long)]
        tag: Option<String>,
    },

    /// パッケージ内容を検証し api.json を生成
    CheckPackage {
        /// 比較対象の旧 api.json
        #[arg(long)]
        previous_api: Option<PathBuf>,

        /// 比較対象の旧 Git tag
        #[arg(long)]
        previous_tag: Option<String>,
    },

    /// 2 つの api.json を比較
    ApiDiff {
        /// 旧 api.json または Git tag
        old: String,

        /// 新 api.json または Git tag
        new: String,
    },

    /// インストール済みパッケージ情報を表示
    Info {
        /// パッケージ名
        package: String,
    },

    /// 対話的 REPL (Read-Eval-Print Loop)
    Repl,

    /// LSP サーバーを起動
    Lsp,

    /// MCP サーバーを起動
    McpServer,

    /// ドキュメント生成 (:doc メタデータから HTML 生成)
    Doc {
        /// 入力ファイル
        file: PathBuf,

        /// 出力ファイル (デフォルト: stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// api.json を生成する
        #[arg(long)]
        json: bool,
    },

    /// guides と stdlib API から静的ドキュメントサイトを生成
    DocSite {
        /// 出力ディレクトリ
        #[arg(short, long, default_value = "_site")]
        output: PathBuf,
    },

    /// L# 開発者向けの Claude Skill / language guide Markdown を標準出力へ表示
    LanguageGuide,

    /// Claude Code へ MCP 設定と L# Skill をインストール
    ClaudePlugin,
}

fn main() -> miette::Result<()> {
    maybe_delegate_to_external_compiler()?;
    maybe_bridge_compile_build_artifact()?;
    maybe_delegate_to_embedded_component()?;
    maybe_hint_shadow_command_requires_lsharp_path()?;

    let cli = Cli::parse();

    match cli.command {
        Command::Init { name } => {
            cmd_init(&name)?;
        }

        Command::Compile {
            file,
            output,
            target,
            backend,
            artifact_cache_dir,
            artifact_cache_max_entries,
            artifact_cache_max_bytes,
            emit_ir,
        }
        | Command::Build {
            file,
            output,
            target,
            backend,
            artifact_cache_dir,
            artifact_cache_max_entries,
            artifact_cache_max_bytes,
            emit_ir,
        } => {
            let artifact_cache_dir = resolve_artifact_cache_dir(artifact_cache_dir)?;
            let (artifact_cache_max_entries, artifact_cache_max_bytes) =
                resolve_artifact_cache_limits(
                    artifact_cache_max_entries,
                    artifact_cache_max_bytes,
                )?;
            validate_artifact_cache_options(
                artifact_cache_dir.as_deref(),
                artifact_cache_max_entries,
                artifact_cache_max_bytes,
            )?;
            // P0-1: git リポジトリ必須チェック
            check_git_repo(&file)?;

            let mut compile_session = artifact_cache_dir
                .clone()
                .map(commands::compile::CompileSession::with_artifact_cache)
                .unwrap_or_else(commands::compile::CompileSession::new);
            let artifacts = compile_session.compile_file_with_backend(
                &file,
                output.as_deref(),
                emit_ir,
                target.map(Into::into),
                backend
                    .map(Into::into)
                    .unwrap_or(commands::compile::CompileBackend::Linear),
            )?;
            maintain_artifact_cache(
                artifact_cache_dir.as_deref(),
                artifact_cache_max_entries,
                artifact_cache_max_bytes,
            )?;
            if !emit_ir {
                print_compile_artifacts_success(&artifacts);
            }
        }

        Command::Test { file } => {
            cmd_test(&file)?;
        }

        Command::Validate {
            file,
            source,
            format,
            emit_manifest,
        } => {
            let exit_code = if let Some(source) = source {
                cmd_validate_source(&source, format, emit_manifest.as_deref())?
            } else {
                let file = resolve_validate_manifest(file)?;
                cmd_validate(&file, format, emit_manifest.as_deref())?
            };
            if exit_code != 0 {
                std::process::exit(exit_code);
            }
        }

        Command::Review { file } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;

            // ドキュメントステータス読み込み
            let status_path = std::path::Path::new(".lsharp-doc-status");
            let doc_status = lsharp_docs::tracker::load_doc_status(status_path);

            // メタデータ検証
            let diag_strings =
                lsharp_tooling::metadata_validation::check_metadata_strings(&source)?;

            // レビューチェックポイント生成
            let checkpoint = lsharp_docs::review::generate_review(
                &file.display().to_string(),
                &source,
                &doc_status,
                &diag_strings,
            );

            // YAML 出力
            print!("{}", lsharp_docs::review::format_yaml(&checkpoint));
        }

        Command::DocAck { name, reviewer } => {
            let status_path = std::path::Path::new(".lsharp-doc-status");
            let mut status = lsharp_docs::tracker::load_doc_status(status_path);
            lsharp_docs::tracker::acknowledge(&mut status, &name, &reviewer);
            lsharp_docs::tracker::save_doc_status(&status, status_path)
                .map_err(|e| driver_io_error(format!("doc-status 保存失敗: {e}")))?;
            println!("'{name}' を確認済みとしてマーク ({reviewer})");
        }

        Command::DocCheck {
            file,
            skip_doc_review,
            emit_trailers,
        } => {
            if skip_doc_review {
                println!("ドキュメントレビューをスキップしました");
                return Ok(());
            }

            let source = std::fs::read_to_string(&file)
                .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;

            // パースとメタデータ検証
            let diagnostics = lsharp_tooling::metadata_validation::check_metadata_strings(&source)?;

            // ドキュメントステータス確認
            let status_path = std::path::Path::new(".lsharp-doc-status");
            let doc_status = lsharp_docs::tracker::load_doc_status(status_path);

            let mut has_errors = false;

            // Stale なドキュメントがあればエラー
            for (name, entry) in &doc_status.entries {
                if entry.freshness == lsharp_docs::tracker::Freshness::Stale {
                    eprintln!(
                        "DOC001: '{}' のドキュメントが古くなっています (lsharp doc-ack {} で確認済みマーク)",
                        name, name
                    );
                    has_errors = true;
                }
            }

            // メタデータ検証エラー（Error レベルのみ）
            for diag_str in &diagnostics {
                if has_metadata_errors(std::slice::from_ref(diag_str)) {
                    eprintln!("DOC002: {}", diag_str);
                    has_errors = true;
                }
            }

            if has_errors {
                return Err(miette::miette!(
                    "ドキュメント検証に失敗しました。\n\
                     `lsharp review {}` で詳細を確認してください。",
                    file.display()
                ));
            }

            if emit_trailers {
                // コミットトレイラー出力
                let trailer_status = if has_errors { "Failed" } else { "Passed" };
                println!("Doc-Review-Status: {trailer_status}");

                // レビュー済みエントリのレビュー者を収集
                let mut reviewers: Vec<String> = doc_status
                    .entries
                    .values()
                    .filter_map(|entry| entry.reviewed_by.clone())
                    .collect();
                reviewers.sort();
                reviewers.dedup();

                if reviewers.is_empty() {
                    println!("Doc-Reviewed-By: none");
                } else {
                    for reviewer in &reviewers {
                        println!("Doc-Reviewed-By: {reviewer}");
                    }
                }
                return Ok(());
            }

            println!("ドキュメント検証OK: {}", file.display());
        }

        Command::Install => {
            cmd_install()?;
        }

        Command::Add { github_url, tag } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| miette::miette!("現在ディレクトリを取得できません: {e}"))?;
            cmd_add_in(&current_dir, &github_url, tag.as_deref())?;
        }

        Command::CheckPackage {
            previous_api,
            previous_tag,
        } => {
            let summary = cmd_check_package_in(
                &std::env::current_dir()
                    .map_err(|e| miette::miette!("current_dir 取得失敗: {e}"))?,
                previous_api.as_deref(),
                previous_tag.as_deref(),
            )?;
            print!("{summary}");
        }

        Command::ApiDiff { old, new } => {
            let summary = cmd_api_diff_specs(
                &std::env::current_dir()
                    .map_err(|e| miette::miette!("current_dir 取得失敗: {e}"))?,
                &old,
                &new,
            )?;
            print!("{summary}");
        }

        Command::Info { package } => {
            let summary = cmd_info_in(
                &std::env::current_dir()
                    .map_err(|e| miette::miette!("current_dir 取得失敗: {e}"))?,
                &package,
            )?;
            print!("{summary}");
        }

        Command::Repl => {
            cmd_repl()?;
        }

        Command::Lsp => {
            tokio::runtime::Runtime::new()
                .map_err(|e| miette::miette!("tokio ランタイム起動失敗: {e}"))?
                .block_on(lsharp_lsp::run_server());
        }

        Command::McpServer => {
            mcp_server::run_stdio_server()?;
        }

        Command::Doc { file, output, json } => {
            cmd_doc(&file, output.as_deref(), json)?;
        }

        Command::DocSite { output } => {
            doc_site::cmd_doc_site(&output)?;
        }

        Command::LanguageGuide => {
            print!("{}", claude_plugin::language_guide_markdown());
        }

        Command::ClaudePlugin => {
            claude_plugin::cmd_claude_plugin()?;
        }
    }

    Ok(())
}

fn print_compile_artifacts_success(artifacts: &commands::compile::CompileArtifacts) {
    let output_size = std::fs::metadata(&artifacts.output_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    println!(
        "コンパイル成功: {} ({} bytes)",
        artifacts.output_path.display(),
        output_size
    );
}

fn driver_config_error(project_dir: &Path, error: config::ConfigError) -> miette::Report {
    match error {
        config::ConfigError::Read(message) => driver_io_error(format!(
            "{}: lsharp.toml の読み込みに失敗: {message}",
            project_dir.join("lsharp.toml").display()
        )),
        other => miette::miette!("{other}"),
    }
}

fn canonicalize_driver_path(path: &Path) -> miette::Result<PathBuf> {
    std::fs::canonicalize(path)
        .map_err(|e| driver_io_error(format!("パスの正規化に失敗 '{}': {e}", path.display())))
}

include!("artifact_cache_options.rs");

fn maybe_delegate_to_embedded_component() -> miette::Result<()> {
    if option_env!("LSHARP_EMBEDDED_COMPONENT_PRESENT") != Some("1") {
        return Ok(());
    }
    if std::env::var_os("LSHARP_DISABLE_EMBEDDED_COMPONENT").is_some_and(|value| value != "0") {
        return Ok(());
    }
    if !should_delegate_to_embedded_component() {
        return Ok(());
    }

    let current_dir =
        std::env::current_dir().map_err(|e| miette::miette!("current dir の取得に失敗: {e}"))?;
    let args =
        normalize_guest_args_for_current_dir(&current_dir, std::env::args().skip(1).collect());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let component_bytes = resolve_default_component_bytes()?;
    let output =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            component_bytes.as_ref(),
            Some(&current_dir),
            &arg_refs,
        )
        .map_err(|e| miette::miette!("embedded component 実行に失敗しました: {e}"))?;
    print!("{}", output.stdout);
    std::process::exit(output.exit_code);
}

fn infer_bridge_compile_target(
    requested_target: Option<commands::compile::CompileTarget>,
    output_path: &Path,
) -> commands::compile::CompileTarget {
    if let Some(target) = requested_target {
        return target;
    }

    let output_name = output_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if output_name.ends_with(".component.wasm") {
        commands::compile::CompileTarget::WasiComponent
    } else if output_path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "wasm")
    {
        commands::compile::CompileTarget::WasiPreview1
    } else {
        commands::compile::CompileTarget::Native
    }
}

fn should_fallback_to_host_compile(guest_exit_code: Option<i32>) -> bool {
    guest_exit_code != Some(0)
}

fn is_selfhost_shadow_command(command: &str) -> bool {
    matches!(command, "parse" | "check" | "test" | "fmt")
}

fn maybe_bridge_compile_build_artifact_with_component(
    component_bytes: &[u8],
) -> miette::Result<bool> {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(_) => return Ok(false),
    };

    let (
        file,
        output,
        target,
        backend,
        artifact_cache_dir,
        artifact_cache_max_entries,
        artifact_cache_max_bytes,
        emit_ir,
    ) = match cli.command {
        Command::Compile {
            file,
            output,
            target,
            backend,
            artifact_cache_dir,
            artifact_cache_max_entries,
            artifact_cache_max_bytes,
            emit_ir,
        }
        | Command::Build {
            file,
            output,
            target,
            backend,
            artifact_cache_dir,
            artifact_cache_max_entries,
            artifact_cache_max_bytes,
            emit_ir,
        } => (
            file,
            output,
            target,
            backend,
            artifact_cache_dir,
            artifact_cache_max_entries,
            artifact_cache_max_bytes,
            emit_ir,
        ),
        _ => return Ok(false),
    };

    if emit_ir
        || backend.is_some()
        || artifact_cache_dir.is_some()
        || artifact_cache_max_entries.is_some()
        || artifact_cache_max_bytes.is_some()
    {
        return Ok(false);
    }

    let current_dir =
        std::env::current_dir().map_err(|e| miette::miette!("current dir の取得に失敗: {e}"))?;
    let args =
        normalize_guest_args_for_current_dir(&current_dir, std::env::args().skip(1).collect());
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let host_file = if file.is_absolute() {
        file.clone()
    } else {
        current_dir.join(&file)
    };
    let requested_target = target.map(Into::into);
    let resolved_output = if let Some(output_path) = output {
        if output_path.is_absolute() {
            output_path
        } else {
            current_dir.join(output_path)
        }
    } else {
        match requested_target.unwrap_or(commands::compile::CompileTarget::WasiComponent) {
            commands::compile::CompileTarget::WasiPreview1
            | commands::compile::CompileTarget::WebWasm => host_file.with_extension("wasm"),
            commands::compile::CompileTarget::WasiComponent => {
                host_file.with_extension("component.wasm")
            }
            commands::compile::CompileTarget::Native => {
                let stem = host_file
                    .file_stem()
                    .map(|stem| stem.to_os_string())
                    .or_else(|| host_file.file_name().map(|name| name.to_os_string()))
                    .unwrap_or_else(|| "a.out".into());
                host_file.with_file_name(stem)
            }
        }
    };
    let host_target = infer_bridge_compile_target(requested_target, &resolved_output);

    let guest_output =
        lsharp_wasm::wasi_runner::run_wasm_component_with_dir_and_args_inherit_stdin_capture(
            component_bytes,
            Some(&current_dir),
            &arg_refs,
        );

    if let Ok(output) = &guest_output
        && !should_fallback_to_host_compile(Some(output.exit_code))
    {
        print!("{}", output.stdout);
        std::process::exit(output.exit_code);
    }

    let mut compile_session = commands::compile::CompileSession::new();
    let artifacts = compile_session.compile_file_with_backend(
        &host_file,
        Some(&resolved_output),
        false,
        Some(host_target),
        commands::compile::CompileBackend::Linear,
    )?;

    match guest_output {
        Ok(_) | Err(_) => {
            print_compile_artifacts_success(&artifacts);
            std::process::exit(0);
        }
    }
}

fn maybe_bridge_compile_build_artifact() -> miette::Result<()> {
    if option_env!("LSHARP_EMBEDDED_COMPONENT_PRESENT") != Some("1") {
        return Ok(());
    }
    if std::env::var_os("LSHARP_PATH").is_some() {
        return Ok(());
    }
    if std::env::var_os("LSHARP_DISABLE_EMBEDDED_COMPONENT").is_some_and(|value| value != "0") {
        return Ok(());
    }
    if !should_delegate_to_embedded_component() {
        return Ok(());
    }

    let component_bytes = resolve_default_component_bytes()?;
    let _ = maybe_bridge_compile_build_artifact_with_component(component_bytes.as_ref())?;
    Ok(())
}

fn canonicalize_for_guest_prefix_match(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = std::fs::canonicalize(path) {
        return Some(canonical);
    }

    let parent = path.parent()?;
    let file_name = path.file_name()?;
    let canonical_parent = std::fs::canonicalize(parent).ok()?;
    Some(canonical_parent.join(file_name))
}

fn normalize_guest_relative_src_path(value: String) -> String {
    let path = Path::new(&value);
    match path.components().next() {
        Some(std::path::Component::Normal(first)) if first == std::ffi::OsStr::new("src") => {
            Path::new(".").join(path).to_string_lossy().into_owned()
        }
        _ => value,
    }
}

fn relativize_guest_path_arg(current_dir: &Path, value: &str) -> String {
    let path = Path::new(value);
    if !path.is_absolute() {
        return normalize_guest_relative_src_path(value.to_string());
    }

    if let Ok(relative) = path.strip_prefix(current_dir) {
        return if relative.as_os_str().is_empty() {
            ".".to_string()
        } else {
            normalize_guest_relative_src_path(relative.to_string_lossy().into_owned())
        };
    }

    let canonical_current_dir =
        std::fs::canonicalize(current_dir).unwrap_or_else(|_| current_dir.to_path_buf());
    let Some(canonical_path) = canonicalize_for_guest_prefix_match(path) else {
        return value.to_string();
    };

    match canonical_path.strip_prefix(&canonical_current_dir) {
        Ok(relative) if !relative.as_os_str().is_empty() => {
            normalize_guest_relative_src_path(relative.to_string_lossy().into_owned())
        }
        Ok(_) => ".".to_string(),
        Err(_) => value.to_string(),
    }
}

fn normalize_guest_args_for_current_dir(current_dir: &Path, args: Vec<String>) -> Vec<String> {
    let mut normalized = args;
    let Some(command) = normalized.first().map(String::as_str) else {
        return normalized;
    };

    let normalize_file_arg = |args: &mut Vec<String>, index: usize| {
        if let Some(value) = args.get(index).cloned()
            && !value.starts_with('-')
        {
            args[index] = relativize_guest_path_arg(current_dir, &value);
        }
    };

    match command {
        "parse" | "check" | "test" | "fmt" | "review" | "doc" | "doc-ack" | "doc-check" => {
            normalize_file_arg(&mut normalized, 1);
        }
        "compile" | "build" => {
            normalize_file_arg(&mut normalized, 1);

            let mut index = 2;
            while index + 1 < normalized.len() {
                match normalized[index].as_str() {
                    "-o" | "--output" => {
                        normalized[index + 1] =
                            relativize_guest_path_arg(current_dir, &normalized[index + 1]);
                        index += 2;
                    }
                    _ => index += 1,
                }
            }
        }
        _ => {}
    }

    normalized
}

fn should_delegate_to_embedded_component() -> bool {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    should_delegate_to_embedded_component_args(&args)
}

fn should_delegate_to_embedded_component_args(args: &[std::ffi::OsString]) -> bool {
    let cache_env = std::env::var_os(ARTIFACT_CACHE_DIR_ENV);
    let cache_limits_env_configured = std::env::var_os(ARTIFACT_CACHE_MAX_ENTRIES_ENV).is_some()
        || std::env::var_os(ARTIFACT_CACHE_MAX_BYTES_ENV).is_some();
    should_delegate_to_embedded_component_args_with_cache_env(
        args,
        cache_env.as_deref(),
        cache_limits_env_configured,
    )
}

fn should_delegate_to_embedded_component_args_with_cache_env(
    args: &[std::ffi::OsString],
    cache_env: Option<&std::ffi::OsStr>,
    cache_limits_env_configured: bool,
) -> bool {
    if (cache_env.is_some() || cache_limits_env_configured)
        && matches!(
            args.first().and_then(|arg| arg.to_str()),
            Some("compile" | "build")
        )
    {
        return false;
    }

    match args.first().and_then(|arg| arg.to_str()) {
        Some("parse" | "check" | "test" | "fmt") => true,
        Some("review") => should_delegate_review_command_args(args),
        Some("doc-ack" | "doc-check") => should_delegate_doc_command_args(args),
        Some("compile" | "build") => should_delegate_compile_build_to_embedded_component_args(args),
        _ => false,
    }
}

fn should_delegate_doc_command_args(args: &[std::ffi::OsString]) -> bool {
    let Some(command) = args.first().and_then(|arg| arg.to_str()) else {
        return false;
    };

    let Some(file_arg) = args.get(1).and_then(|arg| arg.to_str()) else {
        return false;
    };
    if file_arg.starts_with('-') {
        return false;
    }

    match (command, args.len()) {
        ("doc-ack", 2) | ("doc-check", 2) => true,
        ("doc-ack", 3) => matches!(args.get(2).and_then(|arg| arg.to_str()), Some("--trailer")),
        ("doc-check", 3) => matches!(args.get(2).and_then(|arg| arg.to_str()), Some("--strict")),
        _ => false,
    }
}

fn should_delegate_review_command_args(args: &[std::ffi::OsString]) -> bool {
    let Some(file_arg) = args.get(1).and_then(|arg| arg.to_str()) else {
        return false;
    };
    if file_arg.starts_with('-') {
        return false;
    }

    match args.len() {
        2 => true,
        3 => matches!(args.get(2).and_then(|arg| arg.to_str()), Some("--json")),
        4 => matches!(
            (
                args.get(2).and_then(|arg| arg.to_str()),
                args.get(3).and_then(|arg| arg.to_str())
            ),
            (Some("--format"), Some("json"))
        ),
        _ => false,
    }
}

fn should_delegate_compile_build_to_embedded_component_args(args: &[std::ffi::OsString]) -> bool {
    let Some(file_arg) = args.get(1).and_then(|arg| arg.to_str()) else {
        return false;
    };
    if file_arg.starts_with('-') {
        return false;
    }

    let mut index = 2;
    while index < args.len() {
        let Some(flag) = args[index].to_str() else {
            return false;
        };
        match flag {
            "-o" | "--output" => {
                let Some(value) = args.get(index + 1).and_then(|arg| arg.to_str()) else {
                    return false;
                };
                if matches!(
                    value,
                    "-o" | "--output" | "--target" | "--backend" | "--emit-ir" | "-h" | "--help"
                ) {
                    return false;
                }
                index += 2;
            }
            "--target" => {
                let Some(value) = args.get(index + 1).and_then(|arg| arg.to_str()) else {
                    return false;
                };
                if !matches!(value, "wasi-preview1" | "wasi-component" | "wasm") {
                    return false;
                }
                index += 2;
            }
            "--backend" => {
                let Some(value) = args.get(index + 1).and_then(|arg| arg.to_str()) else {
                    return false;
                };
                if !matches!(value, "linear" | "wasmgc") {
                    return false;
                }
                // 明示 backend は embedded component guest が持たないため host 側へ残す。
                return false;
            }
            "--artifact-cache-dir"
            | "--artifact-cache-max-entries"
            | "--artifact-cache-max-bytes" => {
                // artifact cache は Rust host の明示的 filesystem boundary を使う。
                return false;
            }
            "--emit-ir" => return false,
            _ => return false,
        }
    }

    true
}

/// selfhost shadow command が LSHARP_PATH なしで呼ばれた場合、外部 compiler への案内を出す。
/// clap のパース前に argv を直接確認し、ユーザーが LSHARP_PATH を設定するよう誘導する。
fn maybe_hint_shadow_command_requires_lsharp_path() -> miette::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(command) = args.first().and_then(|arg| arg.to_str()) else {
        return Ok(());
    };

    let requires_selfhost = match command {
        "review" => should_delegate_review_command_args(&args),
        "doc-ack" | "doc-check" => should_delegate_doc_command_args(&args),
        _ => is_selfhost_shadow_command(command),
    };

    if requires_selfhost {
        return Err(miette::miette!(
            "サブコマンド '{command}' は現在 selfhost compiler への delegation が必要です。\n\
             LSHARP_PATH 環境変数で外部 lsharp/compiler を指定してください:\n\
             \n\
             LSHARP_PATH=/path/to/selfhost/lsharp lsharp {command} ..."
        ));
    }
    Ok(())
}

fn maybe_delegate_to_external_compiler() -> miette::Result<()> {
    let Some(raw_path) = std::env::var_os("LSHARP_PATH") else {
        return Ok(());
    };

    let configured_path = PathBuf::from(raw_path);
    if configured_path.as_os_str().is_empty() {
        return Err(miette::miette!("LSHARP_PATH が空です"));
    }

    match resolve_external_lsharp_path(&configured_path)? {
        ExternalLsharpPath::Executable(delegate_path) => {
            let current_exe = std::env::current_exe()
                .map_err(|e| miette::miette!("current exe の取得に失敗: {e}"))?;

            let delegate_canonical =
                std::fs::canonicalize(&delegate_path).unwrap_or_else(|_| delegate_path.clone());
            let current_canonical = std::fs::canonicalize(&current_exe).unwrap_or(current_exe);
            if delegate_canonical == current_canonical {
                return Err(miette::miette!(
                    "LSHARP_PATH が現在の lsharp バイナリ自身を指しています: {}",
                    delegate_path.display()
                ));
            }

            let status = std::process::Command::new(&delegate_path)
                .args(std::env::args_os().skip(1))
                .env_remove("LSHARP_PATH")
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| {
                    miette::miette!(
                        "LSHARP_PATH 先の compiler 実行に失敗しました ({}): {e}",
                        delegate_path.display()
                    )
                })?;

            match status.code() {
                Some(code) => std::process::exit(code),
                None => Err(miette::miette!(
                    "LSHARP_PATH 先の compiler がシグナル終了しました: {}",
                    delegate_path.display()
                )),
            }
        }
        ExternalLsharpPath::Wasm(delegate_path) => {
            let wasm_bytes = std::fs::read(&delegate_path).map_err(|e| {
                driver_io_error(format!(
                    "LSHARP_PATH 先の Wasm artifact 読み込みに失敗しました ({}): {e}",
                    delegate_path.display()
                ))
            })?;
            let current_dir = std::env::current_dir()
                .map_err(|e| miette::miette!("current dir の取得に失敗: {e}"))?;
            let args = normalize_guest_args_for_current_dir(
                &current_dir,
                std::env::args().skip(1).collect(),
            );
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let output =
                lsharp_wasm::wasi_runner::run_wasm_with_mode_and_args_inherit_stdin_capture(
                    &wasm_bytes,
                    lsharp_wasm::wasi_runner::WasiMode::Preview1,
                    Some(&current_dir),
                    &arg_refs,
                )
                .map_err(|e| {
                    miette::miette!(
                        "LSHARP_PATH 先の Wasm artifact 実行に失敗しました ({}): {e}",
                        delegate_path.display()
                    )
                })?;
            print!("{}", output.stdout);
            std::process::exit(output.exit_code);
        }
        ExternalLsharpPath::Component(delegate_path) => {
            let component_bytes = std::fs::read(&delegate_path).map_err(|e| {
                driver_io_error(format!(
                    "LSHARP_PATH 先の component artifact 読み込みに失敗しました ({}): {e}",
                    delegate_path.display()
                ))
            })?;
            if should_delegate_compile_build_to_embedded_component_args(
                &std::env::args_os().skip(1).collect::<Vec<_>>(),
            ) {
                let _ = maybe_bridge_compile_build_artifact_with_component(&component_bytes)?;
            }
            let current_dir = std::env::current_dir()
                .map_err(|e| miette::miette!("current dir の取得に失敗: {e}"))?;
            let args = normalize_guest_args_for_current_dir(
                &current_dir,
                std::env::args().skip(1).collect(),
            );
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let output =
                lsharp_wasm::wasi_runner::run_wasm_with_mode_and_args_inherit_stdin_capture(
                    &component_bytes,
                    lsharp_wasm::wasi_runner::WasiMode::Preview2,
                    Some(&current_dir),
                    &arg_refs,
                )
                .map_err(|e| {
                    miette::miette!(
                        "LSHARP_PATH 先の component artifact 実行に失敗しました ({}): {e}",
                        delegate_path.display()
                    )
                })?;
            print!("{}", output.stdout);
            std::process::exit(output.exit_code);
        }
    }
}

enum ExternalLsharpPath {
    Executable(PathBuf),
    Wasm(PathBuf),
    Component(PathBuf),
}

fn resolve_external_lsharp_path(
    configured_path: &std::path::Path,
) -> miette::Result<ExternalLsharpPath> {
    let candidate = if configured_path.is_dir() {
        configured_path.join("lsharp")
    } else {
        configured_path.to_path_buf()
    };

    if !candidate.exists() {
        return Err(miette::miette!(
            "LSHARP_PATH が指す compiler が存在しません: {}",
            candidate.display()
        ));
    }
    if !candidate.is_file() {
        return Err(miette::miette!(
            "LSHARP_PATH が指す compiler は通常ファイルである必要があります: {}",
            candidate.display()
        ));
    }

    if candidate
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".component.wasm"))
    {
        return Ok(ExternalLsharpPath::Component(candidate));
    }

    if candidate
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".wasm"))
    {
        return Ok(ExternalLsharpPath::Wasm(candidate));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mode = std::fs::metadata(&candidate)
            .map_err(|e| {
                miette::miette!(
                    "LSHARP_PATH 先の metadata 取得に失敗しました ({}): {e}",
                    candidate.display()
                )
            })?
            .permissions()
            .mode();
        if mode & 0o111 == 0 {
            return Err(miette::miette!(
                "LSHARP_PATH が指す compiler に実行権限がありません: {}",
                candidate.display()
            ));
        }
    }

    Ok(ExternalLsharpPath::Executable(candidate))
}

/// P3-3: メタデータテスト実行 (:example, :invariant の自動検証)
fn cmd_test(file: &Path) -> miette::Result<()> {
    let run = lsharp_tooling::metadata_test::run_metadata_tests(file)?;

    if !run.has_tests() {
        println!(
            "テストなし: {} にはテスト対象のメタデータがありません",
            file.display()
        );
        return Ok(());
    }

    println!("テスト実行: {} ({} テスト)", file.display(), run.total());

    // 結果表示
    let mut passed = 0;
    let mut failed = 0;

    for result in &run.results {
        let kind_str = lsharp_tooling::metadata_test::test_kind_label(&result.kind);

        if result.passed {
            println!("  PASS: {} ({kind_str})", result.name);
            passed += 1;
        } else {
            let error_msg = result.error.as_deref().unwrap_or("不明なエラー");
            println!("  FAIL: {} ({kind_str}) - {error_msg}", result.name);
            failed += 1;
        }
    }

    println!();
    println!(
        "テスト結果: {} 個中 {} 成功, {} 失敗",
        run.total(),
        passed,
        failed
    );

    if failed > 0 {
        return Err(miette::miette!("{failed} 個のテストが失敗しました"));
    }

    Ok(())
}

fn resolve_validate_manifest(file: Option<PathBuf>) -> miette::Result<PathBuf> {
    if let Some(file) = file {
        return Ok(file);
    }

    let current_dir = std::env::current_dir()
        .map_err(|error| miette::miette!("current dir の取得に失敗: {error}"))?;
    let project_dir = find_project_root(&current_dir);
    let config = config::load_config_result(&project_dir)
        .map_err(|error| miette::miette!("プロジェクト設定の読み込みに失敗しました: {error}"))?;
    config::resolve_validation_manifest_path(&project_dir, config.validation.manifest.as_deref())
        .map_err(|error| miette::miette!("validation manifest の解決に失敗しました: {error}"))
}

/// M2-03: versioned intent/evidence graph manifest の fact-oriented validation。
///
/// `test` の implementation conformance と混同しないよう、pass=0、fail=1、
/// unknown=2 を別の exit code として返す。JSON manifest と source adapter は同じ
/// `IntentGraph` に投影し、report の status/format を共有する。
fn cmd_validate(
    file: &Path,
    format: CliValidationFormat,
    emit_manifest: Option<&Path>,
) -> miette::Result<i32> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let graph = lsharp_types::validation_input::parse_intent_graph_json(&source)
        .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    emit_validation_manifest(&graph, emit_manifest)?;
    emit_validation_report(&graph, format)
}

/// source metadata を intent graph へ投影して fact-oriented validation を実行する。
///
/// source 側では node と node-to-node edge までを受け付ける。contract/evidence が
/// まだ接続されていないため、入力が妥当でも report は unknown を返す。
fn cmd_validate_source(
    file: &Path,
    format: CliValidationFormat,
    emit_manifest: Option<&Path>,
) -> miette::Result<i32> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
    let program = lsharp_syntax::parse(&source)
        .map_err(|e| miette::miette!("[{}] {}: {}", e.code(), file.display(), e))?;
    let graph = lsharp_types::validation_source::source_program_to_intent_graph(&program)
        .map_err(|e| source_graph_error(file, &source, e))?;
    emit_validation_manifest(&graph, emit_manifest)?;
    emit_validation_report(&graph, format)
}

/// source adapter の directive span を CLI の miette 診断へ接続する。
fn source_graph_error(
    file: &Path,
    source: &str,
    error: lsharp_types::validation_source::SourceGraphError,
) -> miette::Report {
    let message = format!("{}: {}", file.display(), error);
    let Some(span) = error.source_span() else {
        return miette::miette!("{message}");
    };
    miette::miette!(
        labels = vec![miette::LabeledSpan::at(
            span.start..span.end,
            "source adapter error"
        )],
        "{message}"
    )
    .with_source_code(miette::NamedSource::new(
        file.display().to_string(),
        source.to_owned(),
    ))
}

fn emit_validation_manifest(
    graph: &lsharp_types::validation::IntentGraph,
    output: Option<&Path>,
) -> miette::Result<()> {
    let Some(output) = output else {
        return Ok(());
    };
    let json = graph
        .to_manifest_json_string()
        .map_err(|e| miette::miette!("validation manifest JSON の生成に失敗しました: {e}"))?;
    atomic_write::write_durable_atomic(output, json.as_bytes())
        .map_err(|e| driver_io_error(format!("{}: {}", output.display(), e)))
}

fn emit_validation_report(
    graph: &lsharp_types::validation::IntentGraph,
    format: CliValidationFormat,
) -> miette::Result<i32> {
    let report = graph.validate();

    match format {
        CliValidationFormat::Text => print!("{}", report.to_text()),
        CliValidationFormat::Json => {
            let json = report
                .to_json_string()
                .map_err(|e| miette::miette!("validation report JSON の生成に失敗しました: {e}"))?;
            println!("{json}");
        }
    }

    Ok(match report.status() {
        lsharp_types::validation::ValidationStatus::Pass => 0,
        lsharp_types::validation::ValidationStatus::Fail => 1,
        lsharp_types::validation::ValidationStatus::Unknown => 2,
    })
}

fn has_metadata_errors(diagnostics: &[String]) -> bool {
    diagnostics.iter().any(|diag| diag.contains("[error]"))
}

/// P0-1: git リポジトリの存在を検証
fn check_git_repo(file: &std::path::Path) -> miette::Result<()> {
    // ファイルの親ディレクトリから .git を探索
    let mut dir = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());

    loop {
        if dir.is_file() {
            dir = dir.parent().unwrap_or(dir.as_ref()).to_path_buf();
        }

        let git_dir = dir.join(".git");
        if git_dir.exists() {
            return Ok(());
        }

        if let Some(parent) = dir.parent() {
            if parent == dir {
                break;
            }
            dir = parent.to_path_buf();
        } else {
            break;
        }
    }

    Err(miette::miette!(
        "PROJ001: git リポジトリが見つかりません。\n\
         `lsharp init <project-name>` でプロジェクトを初期化するか、\n\
         `git init` を実行してください。"
    ))
}

/// P0-2: lsharp init コマンドの実装
fn cmd_init(name: &str) -> miette::Result<()> {
    cmd_init_in(Path::new("."), name)
}

fn cmd_init_in(base_dir: &Path, name: &str) -> miette::Result<()> {
    use std::fs;
    use std::process::Command as ProcessCommand;

    let project_dir = base_dir.join(name);

    // ディレクトリ作成
    if project_dir.exists() {
        return Err(miette::miette!("ディレクトリ '{name}' は既に存在します"));
    }

    fs::create_dir_all(&project_dir)
        .map_err(|e| driver_io_error(format!("ディレクトリ作成失敗: {e}")))?;

    // 標準ディレクトリ作成
    let src_dir = project_dir.join("src");
    let examples_dir = project_dir.join("examples");
    let tests_dir = project_dir.join("tests");
    let docs_dir = project_dir.join("docs");
    fs::create_dir_all(&src_dir)
        .map_err(|e| driver_io_error(format!("src ディレクトリ作成失敗: {e}")))?;
    fs::create_dir_all(&examples_dir)
        .map_err(|e| driver_io_error(format!("examples ディレクトリ作成失敗: {e}")))?;
    fs::create_dir_all(&tests_dir)
        .map_err(|e| driver_io_error(format!("tests ディレクトリ作成失敗: {e}")))?;
    fs::create_dir_all(&docs_dir)
        .map_err(|e| driver_io_error(format!("docs ディレクトリ作成失敗: {e}")))?;

    // lsharp.toml 生成
    let toml_content = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
entry = "src/Main.ls"
"#
    );
    fs::write(project_dir.join("lsharp.toml"), toml_content)
        .map_err(|e| driver_io_error(format!("lsharp.toml 作成失敗: {e}")))?;

    // Main.ls 生成
    let main_content = r#"(module Main)

(defn main []
  (print 42))
"#;
    fs::write(src_dir.join("Main.ls"), main_content)
        .map_err(|e| driver_io_error(format!("Main.ls 作成失敗: {e}")))?;

    // .gitignore 生成
    let gitignore_content = "*.wasm\n/target/\n/.lsharp/\n";
    fs::write(project_dir.join(".gitignore"), gitignore_content)
        .map_err(|e| driver_io_error(format!(".gitignore 作成失敗: {e}")))?;

    // git init
    let git_result = ProcessCommand::new("git")
        .arg("init")
        .current_dir(&project_dir)
        .output();

    match git_result {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(miette::miette!("git init 失敗: {stderr}"));
        }
        Err(e) => {
            return Err(miette::miette!(
                "git コマンドが見つかりません: {e}\n\
                 git をインストールしてください。"
            ));
        }
    }

    // 初期コミット
    let _ = ProcessCommand::new("git")
        .args(["add", "."])
        .current_dir(&project_dir)
        .output();

    let _ = ProcessCommand::new("git")
        .args(["commit", "-m", "Initial L# project"])
        .current_dir(&project_dir)
        .output();

    println!("プロジェクト '{name}' を作成しました");
    println!("  {}/lsharp.toml", name);
    println!("  {}/src/Main.ls", name);
    println!("\n次のステップ:");
    println!("  cd {name}");
    println!("  lsharp compile src/Main.ls");

    Ok(())
}

/// P9-4 / P12-A1: ドキュメント生成
fn cmd_doc(file: &Path, output: Option<&Path>, json: bool) -> miette::Result<()> {
    if json {
        return cmd_doc_json(file, output);
    }

    let html = lsharp_tooling::doc_html::render_doc_html(file)?;

    if let Some(output_path) = output {
        std::fs::write(output_path, &html)
            .map_err(|e| driver_io_error(format!("{}: {}", output_path.display(), e)))?;
        println!("ドキュメント生成: {}", output_path.display());
    } else {
        print!("{}", html);
    }

    Ok(())
}

fn cmd_doc_json(file: &Path, output: Option<&Path>) -> miette::Result<()> {
    let (project_root, package, version) = package_identity_for_file(file);
    let api = if project_root.join("src").is_dir() {
        api_doc::build_api_doc_for_package(&project_root, &package, &version)?
    } else {
        api_doc::build_api_doc_for_file(&package, &version, file)?
    };
    let json = serde_json::to_string_pretty(&api)
        .map_err(|e| miette::miette!("api.json の直列化に失敗: {e}"))?;

    let output_path = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| project_root.join("docs").join("api.json"));
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| driver_io_error(format!("{}: {}", parent.display(), e)))?;
    }
    std::fs::write(&output_path, json)
        .map_err(|e| driver_io_error(format!("{}: {}", output_path.display(), e)))?;
    println!("Generated {}", output_path.display());

    Ok(())
}

fn package_identity_for_file(file: &Path) -> (PathBuf, String, String) {
    let start = file.parent().unwrap_or_else(|| Path::new("."));
    let project_root = find_project_root(start);
    let config = config::load_config(&project_root);
    let package = if config.project.name.is_empty() {
        file.file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        config.project.name
    };
    let version = if config.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        config.project.version
    };
    (project_root, package, version)
}

fn find_project_root(start: &Path) -> PathBuf {
    let mut current = start.to_path_buf();
    loop {
        if current.join("lsharp.toml").exists() {
            return current;
        }
        let Some(parent) = current.parent() else {
            return start.to_path_buf();
        };
        if parent == current {
            return start.to_path_buf();
        }
        current = parent.to_path_buf();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiDiffSummary {
    added: Vec<String>,
    changed: Vec<String>,
    removed: Vec<String>,
}

fn cmd_check_package_in(
    project_dir: &Path,
    previous_api: Option<&Path>,
    previous_tag: Option<&str>,
) -> miette::Result<String> {
    let mut out = String::new();
    out.push_str("Validating lsharp.toml ... ");
    let config =
        config::load_config_result(project_dir).map_err(|e| driver_config_error(project_dir, e))?;
    out.push_str("ok\n");

    let package = if config.project.name.is_empty() {
        project_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        config.project.name.clone()
    };
    let version = if config.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        config.project.version.clone()
    };
    let api = api_doc::build_api_doc_for_package(project_dir, &package, &version)?;
    let api_json = serde_json::to_vec_pretty(&api)
        .map_err(|e| miette::miette!("api.json の直列化に失敗: {e}"))?;
    let docs_dir = project_dir.join("docs");
    std::fs::create_dir_all(&docs_dir)
        .map_err(|e| driver_io_error(format!("{}: {}", docs_dir.display(), e)))?;
    let api_path = docs_dir.join("api.json");
    std::fs::write(&api_path, &api_json)
        .map_err(|e| driver_io_error(format!("{}: {}", api_path.display(), e)))?;
    out.push_str("Generating api.json ... ok\n");

    if let Some((label, previous_doc)) =
        resolve_previous_api_doc(project_dir, previous_api, previous_tag)?
    {
        let diff = diff_api_docs(&previous_doc, &api);
        out.push_str(&format!("Comparing with {label} ...\n"));
        out.push_str(&render_diff_lines(&diff));
    }

    let checksum = sha256_hex(&api_json);
    out.push_str(&format!("checksum: sha256:{checksum}\n"));
    Ok(out)
}

fn cmd_api_diff_specs(project_dir: &Path, old: &str, new: &str) -> miette::Result<String> {
    let old_api = read_api_doc_spec(project_dir, old)?;
    let new_api = read_api_doc_spec(project_dir, new)?;
    let diff = diff_api_docs(&old_api, &new_api);
    Ok(render_diff_summary(&diff))
}

fn cmd_info_in(project_dir: &Path, package: &str) -> miette::Result<String> {
    let package_dir = find_installed_package_dir(project_dir, package)
        .ok_or_else(|| miette::miette!("インストール済みパッケージが見つかりません: {package}"))?;
    let api = read_or_generate_api_doc(&package_dir)?;
    let source = read_package_source(project_dir, package).unwrap_or_else(|| "unknown".to_string());

    let module_names: Vec<&str> = api
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    let mut out = String::new();
    out.push_str(&format!("Package: {}@{}\n", api.package, api.version));
    out.push_str(&format!("Source: {source}\n"));
    out.push_str(&format!("Modules: {}\n", module_names.join(", ")));
    out.push_str("Functions:\n");
    for module in &api.modules {
        for function in &module.functions {
            let doc = function.doc.as_deref().unwrap_or("");
            out.push_str(&format!(
                "  {}.{} : {}",
                module.name, function.name, function.signature
            ));
            if !doc.is_empty() {
                out.push_str(&format!(" - {doc}"));
            }
            out.push('\n');
        }
    }
    if api.modules.iter().all(|module| module.functions.is_empty()) {
        out.push_str("  (none)\n");
    }
    out.push_str("Types:\n");
    let mut any_type = false;
    for module in &api.modules {
        for item in &module.types {
            any_type = true;
            out.push_str(&format!(
                "  {}.{} : {}\n",
                module.name, item.name, item.kind
            ));
        }
    }
    if !any_type {
        out.push_str("  (none)\n");
    }
    Ok(out)
}

fn read_api_doc(path: &Path) -> miette::Result<api_doc::ApiDoc> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| driver_io_error(format!("{}: {}", path.display(), e)))?;
    serde_json::from_str(&content).map_err(|e| miette::miette!("{}: {}", path.display(), e))
}

fn read_api_doc_spec(project_dir: &Path, spec: &str) -> miette::Result<api_doc::ApiDoc> {
    let candidate = Path::new(spec);
    if candidate.exists() {
        return read_api_doc(candidate);
    }
    let relative = project_dir.join(spec);
    if relative.exists() {
        return read_api_doc(&relative);
    }
    read_api_doc_from_git_ref(project_dir, spec)
}

fn read_api_doc_from_git_ref(project_dir: &Path, git_ref: &str) -> miette::Result<api_doc::ApiDoc> {
    let path_spec = format!("{git_ref}:docs/api.json");
    let content =
        git_stdout(project_dir, &["show", &path_spec]).map_err(|e| miette::miette!("{e}"))?;
    serde_json::from_str(&content).map_err(|e| miette::miette!("{git_ref}:docs/api.json: {e}"))
}

fn read_or_generate_api_doc(package_dir: &Path) -> miette::Result<api_doc::ApiDoc> {
    let api_path = package_dir.join("docs").join("api.json");
    if api_path.exists() {
        return read_api_doc(&api_path);
    }

    let config = config::load_config(package_dir);
    let package = if config.project.name.is_empty() {
        package_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        config.project.name
    };
    let version = if config.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        config.project.version
    };
    api_doc::build_api_doc_for_package(package_dir, &package, &version)
}

fn read_package_source(project_dir: &Path, package: &str) -> Option<String> {
    let lock_path = project_dir.join(".lsharp").join("lock.toml");
    let lockfile = lockfile::read_lockfile(&lock_path).ok()?;
    lockfile
        .entries
        .into_iter()
        .find(|entry| entry.name == package)
        .map(|entry| entry.source)
}

fn resolve_previous_api_doc(
    project_dir: &Path,
    previous_api: Option<&Path>,
    previous_tag: Option<&str>,
) -> miette::Result<Option<(String, api_doc::ApiDoc)>> {
    if let Some(path) = previous_api {
        return Ok(Some((path.display().to_string(), read_api_doc(path)?)));
    }
    if let Some(tag) = previous_tag {
        return Ok(Some((
            tag.to_string(),
            read_api_doc_from_git_ref(project_dir, tag)?,
        )));
    }
    if let Some(tag) = latest_git_tag(project_dir) {
        let api = read_api_doc_from_git_ref(project_dir, &tag)?;
        return Ok(Some((tag, api)));
    }
    Ok(None)
}

fn latest_git_tag(project_dir: &Path) -> Option<String> {
    let output = git_stdout(project_dir, &["tag", "--sort=-creatordate"]).ok()?;
    output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn diff_api_docs(old: &api_doc::ApiDoc, new: &api_doc::ApiDoc) -> ApiDiffSummary {
    let old_functions = flatten_api_functions(old);
    let new_functions = flatten_api_functions(new);
    let old_types = flatten_api_types(old);
    let new_types = flatten_api_types(new);

    let mut added = Vec::new();
    let mut changed = Vec::new();
    let mut removed = Vec::new();

    for (name, signature) in &new_functions {
        match old_functions.get(name) {
            None => added.push(format!("+ {name} : {signature}")),
            Some(old_signature) if old_signature != signature => changed.push(format!(
                "~ {name} : {old_signature} -> {signature}  BREAKING"
            )),
            _ => {}
        }
    }
    for name in old_functions.keys() {
        if !new_functions.contains_key(name) {
            removed.push(format!("- {name}"));
        }
    }

    for (name, kind) in &new_types {
        match old_types.get(name) {
            None => added.push(format!("+ {name} : type {kind}")),
            Some(old_kind) if old_kind != kind => {
                changed.push(format!("~ {name} : {old_kind} -> {kind}  BREAKING"))
            }
            _ => {}
        }
    }
    for name in old_types.keys() {
        if !new_types.contains_key(name) {
            removed.push(format!("- {name}"));
        }
    }

    added.sort();
    changed.sort();
    removed.sort();

    ApiDiffSummary {
        added,
        changed,
        removed,
    }
}

fn flatten_api_functions(api: &api_doc::ApiDoc) -> std::collections::BTreeMap<String, String> {
    let mut functions = std::collections::BTreeMap::new();
    for module in &api.modules {
        for function in &module.functions {
            functions.insert(
                format!("{}.{}", module.name, function.name),
                function.signature.clone(),
            );
        }
    }
    functions
}

fn flatten_api_types(api: &api_doc::ApiDoc) -> std::collections::BTreeMap<String, String> {
    let mut types = std::collections::BTreeMap::new();
    for module in &api.modules {
        for item in &module.types {
            types.insert(format!("{}.{}", module.name, item.name), item.kind.clone());
        }
    }
    types
}

fn render_diff_summary(diff: &ApiDiffSummary) -> String {
    let mut out = String::new();
    if diff.added.is_empty() {
        out.push_str("Added:    (none)\n");
    } else {
        for line in &diff.added {
            out.push_str(&format!("Added:    {line}\n"));
        }
    }
    if diff.changed.is_empty() {
        out.push_str("Changed:  (none)\n");
    } else {
        for line in &diff.changed {
            out.push_str(&format!("Changed:  {line}\n"));
        }
    }
    if diff.removed.is_empty() {
        out.push_str("Removed:  (none)\n");
    } else {
        for line in &diff.removed {
            out.push_str(&format!("Removed:  {line}\n"));
        }
    }
    out
}

fn render_diff_lines(diff: &ApiDiffSummary) -> String {
    let mut out = String::new();
    for line in &diff.added {
        out.push_str(&format!("  {line}\n"));
    }
    for line in &diff.changed {
        out.push_str(&format!("  {line}\n"));
    }
    for line in &diff.removed {
        out.push_str(&format!("  {line}\n"));
    }
    if out.is_empty() {
        out.push_str("  (no changes)\n");
    }
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// 依存パッケージをインストール
///
/// lsharp.toml の [dependencies] セクションを読み込み、
/// Path/Git 依存を `.lsharp/packages/<name>-<hash>/` に配置する。
fn cmd_install() -> miette::Result<()> {
    cmd_install_in(Path::new("."))
}

fn normalize_github_dependency(input: &str) -> miette::Result<(String, String)> {
    let trimmed = input.trim().trim_end_matches('/');
    let path = if let Some(path) = trimmed.strip_prefix("https://github.com/") {
        path
    } else if let Some(path) = trimmed.strip_prefix("github.com/") {
        path
    } else {
        return Err(miette::miette!(
            "GitHub URL は 'github.com/user/repo' 形式で指定してください: {input}"
        ));
    };

    let repo_path = path.trim_end_matches(".git");
    let mut parts = repo_path.split('/');
    let Some(owner) = parts.next() else {
        return Err(miette::miette!("GitHub URL の owner が不正です: {input}"));
    };
    let Some(repo) = parts.next() else {
        return Err(miette::miette!(
            "GitHub URL の repository が不正です: {input}"
        ));
    };
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        return Err(miette::miette!(
            "GitHub URL は 'github.com/user/repo' 形式で指定してください: {input}"
        ));
    }

    Ok((
        repo.to_string(),
        format!("https://github.com/{owner}/{repo}.git"),
    ))
}

fn cmd_add_in(project_dir: &Path, github_url: &str, tag: Option<&str>) -> miette::Result<()> {
    let config_path = project_dir.join("lsharp.toml");
    if !config_path.exists() {
        return Err(miette::miette!(
            "lsharp.toml が見つかりません: {}",
            config_path.display()
        ));
    }

    let (package_name, git_url) = normalize_github_dependency(github_url)?;
    let config = config::load_config(project_dir);
    if config.dependencies.contains_key(&package_name) {
        return Err(miette::miette!(
            "依存 '{}' は既に lsharp.toml に存在します",
            package_name
        ));
    }

    let mut content = std::fs::read_to_string(&config_path)
        .map_err(|e| driver_io_error(format!("{}: {}", config_path.display(), e)))?;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push('\n');
    content.push_str(&format!("[dependencies.{package_name}]\n"));
    content.push_str(&format!("git = \"{git_url}\"\n"));
    if let Some(tag) = tag {
        content.push_str(&format!("tag = \"{tag}\"\n"));
    }

    std::fs::write(&config_path, content)
        .map_err(|e| driver_io_error(format!("{}: {}", config_path.display(), e)))?;
    println!("Added {package_name} to lsharp.toml");
    Ok(())
}

/// 指定ディレクトリを基点に依存パッケージをインストール (テスト用に分離)
fn cmd_install_in(project_dir: &Path) -> miette::Result<()> {
    let config =
        config::load_config_result(project_dir).map_err(|e| driver_config_error(project_dir, e))?;

    let deps = &config.dependencies;

    let lsharp_dir = project_dir.join(".lsharp");
    let packages_dir = lsharp_dir.join("packages");
    std::fs::create_dir_all(&packages_dir)
        .map_err(|e| driver_io_error(format!(".lsharp/packages/ の作成に失敗: {e}")))?;

    if deps.is_empty() {
        rebuild_installed_module_index(project_dir)?;
        println!("依存パッケージはありません");
        return Ok(());
    }

    let mut installed = 0u32;
    let mut skipped = 0u32;
    let mut resolved_entries = Vec::new();

    for (name, spec) in deps {
        match spec {
            config::DependencySpec::Path { path } => {
                let resolved = project_dir.join(path);
                if !resolved.exists() {
                    eprintln!(
                        "警告: パス依存 '{name}' のパスが存在しません: {}",
                        resolved.display()
                    );
                    skipped += 1;
                    continue;
                }
                let toml_path = resolved.join("lsharp.toml");
                if !toml_path.exists() {
                    eprintln!(
                        "警告: パス依存 '{name}' に lsharp.toml が見つかりません: {}",
                        resolved.display()
                    );
                    skipped += 1;
                    continue;
                }

                let abs_resolved = canonicalize_driver_path(&resolved)?;
                let source_id = dependency_source_string(spec, project_dir);
                let link_path = installed_package_dir(&packages_dir, name, &source_id);
                // 既存のシンボリックリンクがあれば削除
                if link_path.exists() || link_path.symlink_metadata().is_ok() {
                    std::fs::remove_file(&link_path)
                        .or_else(|_| std::fs::remove_dir_all(&link_path))
                        .map_err(|e| driver_io_error(format!("既存リンクの削除に失敗: {e}")))?;
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&abs_resolved, &link_path).map_err(|e| {
                    driver_io_error(format!("シンボリックリンク作成に失敗 '{name}': {e}"))
                })?;

                #[cfg(not(unix))]
                std::fs::copy(&abs_resolved, &link_path)
                    .map_err(|e| driver_io_error(format!("依存コピーに失敗 '{name}': {e}")))?;

                let _ = generate_api_json_for_package(&link_path);
                println!("  インストール: {name} -> {}", abs_resolved.display());
                resolved_entries.push(lockfile::LockEntry {
                    name: name.clone(),
                    version: resolver::package_version_text(&abs_resolved),
                    source: dependency_source_string(spec, project_dir),
                });
                installed += 1;
            }
            config::DependencySpec::Git { git, branch, tag } => {
                let source_id = dependency_source_string(spec, project_dir);
                let dep_path = installed_package_dir(&packages_dir, name, &source_id);
                if dep_path.exists() {
                    println!("  already installed: {name}");
                    resolved_entries.push(lockfile::LockEntry {
                        name: name.clone(),
                        version: resolver::package_version_text(&dep_path),
                        source: dependency_source_string(spec, project_dir),
                    });
                    skipped += 1;
                    continue;
                }

                let clone_result = git_clone(git, branch.as_deref(), tag.as_deref(), &dep_path);
                match clone_result {
                    Ok(()) => {
                        let _ = generate_api_json_for_package(&dep_path);
                        println!("  インストール: {name} (git: {git})");
                        resolved_entries.push(lockfile::LockEntry {
                            name: name.clone(),
                            version: resolver::package_version_text(&dep_path),
                            source: dependency_source_string(spec, project_dir),
                        });
                        installed += 1;
                    }
                    Err(e) => {
                        eprintln!("  失敗: {name} (git clone エラー: {e})");
                        skipped += 1;
                    }
                }
            }
            config::DependencySpec::Version(v) => {
                let resolved = resolver::resolve_cached_version_dependency(project_dir, name, v)
                    .map_err(|e| miette::miette!("{e}"))?;
                println!("  解決: {name}@{} (cached)", resolved.version);
                resolved_entries.push(lockfile::LockEntry {
                    name: name.clone(),
                    version: resolved.version,
                    source: "registry:default".to_string(),
                });
                installed += 1;
            }
        }
    }

    println!("\nインストール完了: {installed} 個インストール, {skipped} 個スキップ");

    // ロックファイルを生成・書き出し
    let lock = lockfile::generate_lockfile_from_entries(resolved_entries);
    std::fs::create_dir_all(&lsharp_dir)
        .map_err(|e| driver_io_error(format!("{}: {}", lsharp_dir.display(), e)))?;
    let lock_path = lsharp_dir.join("lock.toml");
    lockfile::write_lockfile(&lock, &lock_path)
        .map_err(|e| driver_io_error(format!("{}: {e}", lock_path.display())))?;
    println!("ロックファイルを生成しました: {}", lock_path.display());
    rebuild_installed_module_index(project_dir)?;

    Ok(())
}

fn dependency_source_string(spec: &config::DependencySpec, project_dir: &Path) -> String {
    match spec {
        config::DependencySpec::Path { path } => {
            let resolved = project_dir.join(path);
            let resolved = resolved.canonicalize().unwrap_or(resolved);
            format!("path:{}", resolved.display())
        }
        config::DependencySpec::Git { git, branch, tag } => {
            let ref_part = if let Some(branch) = branch {
                format!("?branch={branch}")
            } else if let Some(tag) = tag {
                format!("?tag={tag}")
            } else {
                String::new()
            };
            format!("git:{git}{ref_part}")
        }
        config::DependencySpec::Version(version) => format!("registry:{version}"),
    }
}

fn installed_package_dir(packages_dir: &Path, name: &str, source_id: &str) -> PathBuf {
    let hash = stable_hash_hex(source_id);
    packages_dir.join(format!("{name}-{}", &hash[..8]))
}

fn stable_hash_hex(input: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn generate_api_json_for_package(package_root: &Path) -> miette::Result<Option<PathBuf>> {
    if !package_root.join("src").is_dir() {
        return Ok(None);
    }

    let config = config::load_config(package_root);
    let package = if config.project.name.is_empty() {
        package_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("package")
            .to_string()
    } else {
        config.project.name
    };
    let version = if config.project.version.is_empty() {
        "0.1.0".to_string()
    } else {
        config.project.version
    };
    let api = api_doc::build_api_doc_for_package(package_root, &package, &version)?;
    let json = serde_json::to_string_pretty(&api)
        .map_err(|e| miette::miette!("api.json の直列化に失敗: {e}"))?;
    let docs_dir = package_root.join("docs");
    std::fs::create_dir_all(&docs_dir)
        .map_err(|e| driver_io_error(format!("{}: {}", docs_dir.display(), e)))?;
    let output_path = docs_dir.join("api.json");
    std::fs::write(&output_path, json)
        .map_err(|e| driver_io_error(format!("{}: {}", output_path.display(), e)))?;
    Ok(Some(output_path))
}

fn rebuild_installed_module_index(project_dir: &Path) -> miette::Result<()> {
    let index_root = project_dir.join(".lsharp").join("module-index");
    if index_root.exists() {
        std::fs::remove_dir_all(&index_root)
            .map_err(|e| driver_io_error(format!("{}: {}", index_root.display(), e)))?;
    }

    let package_dirs = list_installed_package_dirs(project_dir);
    for package_dir in package_dirs {
        write_package_module_index(project_dir, &index_root, &package_dir)?;
    }
    Ok(())
}

fn write_package_module_index(
    project_dir: &Path,
    index_root: &Path,
    package_dir: &Path,
) -> miette::Result<()> {
    let source_root = package_dir.join("src");
    if !source_root.is_dir() {
        return Ok(());
    }

    let config = config::load_config(package_dir);
    let exports = if config.project.exports.modules.is_empty() {
        None
    } else {
        Some(
            config
                .project
                .exports
                .modules
                .into_iter()
                .collect::<BTreeSet<_>>(),
        )
    };

    let mut files = Vec::new();
    collect_package_source_files(&source_root, &mut files)?;
    files.sort();

    for file in files {
        let Some(module_name) = module_name_for_source_file(&source_root, &file) else {
            continue;
        };
        if exports
            .as_ref()
            .is_some_and(|allowed| !allowed.contains(&module_name))
        {
            continue;
        }

        let index_rel = format!("{}.path", module_name.replace('.', "/"));
        let index_path = index_root.join(index_rel);
        if index_path.exists() {
            continue;
        }
        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| driver_io_error(format!("{}: {}", parent.display(), e)))?;
        }

        let target = file
            .strip_prefix(project_dir)
            .unwrap_or(file.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        std::fs::write(&index_path, target)
            .map_err(|e| driver_io_error(format!("{}: {}", index_path.display(), e)))?;
    }

    Ok(())
}

fn collect_package_source_files(dir: &Path, out: &mut Vec<PathBuf>) -> miette::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries =
        std::fs::read_dir(dir).map_err(|e| driver_io_error(format!("{}: {}", dir.display(), e)))?;
    for entry in entries {
        let entry = entry.map_err(|e| driver_io_error(format!("{}: {}", dir.display(), e)))?;
        let path = entry.path();
        if path.is_dir() {
            collect_package_source_files(&path, out)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("ls") {
            out.push(path);
        }
    }
    Ok(())
}

fn module_name_for_source_file(source_root: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(source_root).ok()?;
    let stem = relative.with_extension("");
    let parts: Option<Vec<&str>> = stem.iter().map(|part| part.to_str()).collect();
    let parts = parts?;
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("."))
    }
}

fn list_installed_package_dirs(project_dir: &Path) -> Vec<PathBuf> {
    let packages_dir = project_dir.join(".lsharp").join("packages");
    let Ok(entries) = std::fs::read_dir(packages_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir() || path.symlink_metadata().is_ok())
        .collect();
    paths.sort();
    paths
}

fn find_installed_package_dir(project_dir: &Path, name: &str) -> Option<PathBuf> {
    list_installed_package_dirs(project_dir)
        .into_iter()
        .find(|path| {
            path.file_name()
                .and_then(|entry| entry.to_str())
                .is_some_and(|entry| entry.starts_with(&format!("{name}-")))
        })
}

/// Git リポジトリをクローンする
///
/// shallow clone (--depth 1) で高速にクローンする。
/// branch または tag が指定されている場合は --branch オプションを付与する。
fn git_clone(
    url: &str,
    branch: Option<&str>,
    tag: Option<&str>,
    dest: &std::path::Path,
) -> Result<(), String> {
    let mut args = vec!["clone", "--depth", "1"];

    // branch が優先、なければ tag を使用
    let ref_spec = branch.or(tag);
    if let Some(r) = ref_spec {
        args.push("--branch");
        args.push(r);
    }

    args.push(url);
    let dest_str = dest.to_string_lossy();
    args.push(&dest_str);

    let output = std::process::Command::new("git")
        .args(&args)
        .output()
        .map_err(|e| format!("git コマンドの実行に失敗: {e}"))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git clone 失敗: {}", stderr.trim()))
    }
}

fn git_stdout(project_dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("git コマンドの実行に失敗: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git 実行失敗: {}", stderr.trim()));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("git stdout の UTF-8 変換に失敗: {e}"))
}

/// P9-1: 対話的 REPL
fn cmd_repl() -> miette::Result<()> {
    use rustyline::DefaultEditor;
    use rustyline::error::ReadlineError;

    println!("L# REPL v0.1.0");
    println!("式を入力してください。終了するには Ctrl+D を押してください。");
    println!();

    let mut rl = DefaultEditor::new().expect("readline の初期化に失敗しました");

    // 履歴ファイルの読み込み (存在しなくても無視)
    let history_path = dirs::home_dir()
        .map(|h| h.join(".lsharp_history"))
        .unwrap_or_else(|| std::path::PathBuf::from(".lsharp_history"));
    let _ = rl.load_history(&history_path);

    let mut expr_count = 0;

    loop {
        match rl.readline("lsharp> ") {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                rl.add_history_entry(line).unwrap_or_default();

                match lsharp_tooling::repl::evaluate_expression(line) {
                    Ok(output) => {
                        let output = output.trim();
                        if !output.is_empty() {
                            println!("{}", output);
                        }
                        expr_count += 1;
                    }
                    Err(err) => eprintln!("{err}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C: 現在の入力をキャンセルして続行
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D: REPL を終了
                println!();
                break;
            }
            Err(e) => {
                eprintln!("入力エラー: {}", e);
                break;
            }
        }
    }

    // 履歴ファイルの保存
    let _ = rl.save_history(&history_path);

    println!("セッション終了。{} 式を評価しました。", expr_count);
    Ok(())
}
