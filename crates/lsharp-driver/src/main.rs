//! # Default path / compiler path (OPS-05)
//!
//! - 現行: 本バイナリが Rust 実装パイプライン（syntax → types → ir → wasm）を**内蔵**する。
//! - 移行中: 環境変数 `LSHARP_PATH` で selfhost / 外部コンパイラ executable・その配置ディレクトリ・`.wasm` / `.component.wasm` guest artifact を指せる。
//! - 検証: `scripts/ci/default-path-smoke.sh` が `target/debug/lsharp` 単体で embedded default path の `compile` / `build` を含む smoke を通す。

mod api_doc;
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

use clap::{Parser, Subcommand, ValueEnum};
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
        miette::miette!(
            "adjacent component sidecar の読み込みに失敗しました ({}): {e}",
            sidecar_path.display()
        )
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

        /// IR を表示する
        #[arg(long)]
        emit_ir: bool,
    },

    /// メタデータテストを実行 (:example, :invariant の自動検証)
    Test {
        /// 入力ファイル
        file: PathBuf,
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
            emit_ir,
        }
        | Command::Build {
            file,
            output,
            target,
            backend,
            emit_ir,
        } => {
            // P0-1: git リポジトリ必須チェック
            check_git_repo(&file)?;

            let artifacts = commands::compile::compile_file_with_backend(
                &file,
                output.as_deref(),
                emit_ir,
                target.map(Into::into),
                backend
                    .map(Into::into)
                    .unwrap_or(commands::compile::CompileBackend::Linear),
            )?;
            if !emit_ir {
                print_compile_artifacts_success(&artifacts);
            }
        }

        Command::Test { file } => {
            cmd_test(&file)?;
        }

        Command::Review { file } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

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
                .map_err(|e| miette::miette!("doc-status 保存失敗: {e}"))?;
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
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

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

    let (file, output, target, backend, emit_ir) = match cli.command {
        Command::Compile {
            file,
            output,
            target,
            backend,
            emit_ir,
        }
        | Command::Build {
            file,
            output,
            target,
            backend,
            emit_ir,
        } => (file, output, target, backend, emit_ir),
        _ => return Ok(false),
    };

    if emit_ir || backend.is_some() {
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

    let artifacts = commands::compile::compile_file_with_backend(
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
                miette::miette!(
                    "LSHARP_PATH 先の Wasm artifact 読み込みに失敗しました ({}): {e}",
                    delegate_path.display()
                )
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
                miette::miette!(
                    "LSHARP_PATH 先の component artifact 読み込みに失敗しました ({}): {e}",
                    delegate_path.display()
                )
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

fn has_metadata_errors(diagnostics: &[String]) -> bool {
    diagnostics.iter().any(|diag| diag.contains("[error]"))
}

/// Knowledge JSON を構築
#[allow(dead_code)]
#[cfg(test)]
fn build_knowledge(
    program: &lsharp_syntax::ast::Program,
    type_results: &[(String, lsharp_types::types::TypeScheme)],
    infer: &lsharp_types::infer::Infer,
) -> lsharp_docs::knowledge::Knowledge {
    use lsharp_docs::knowledge::*;
    use lsharp_syntax::ast::Decl;

    let module_name = infer.module_env.name.clone();

    let mut functions = Vec::new();
    let mut types = Vec::new();
    let is_private_set: std::collections::HashSet<&str> = infer
        .module_env
        .privates
        .iter()
        .map(|s| s.as_str())
        .collect();

    // Private 名の収集のために再利用
    let _ = &is_private_set;

    for decl in &program.decls {
        // Private を展開
        let (actual_decl, is_priv) = match decl {
            Decl::Private { inner, .. } => (inner.as_ref(), true),
            other => (other, false),
        };

        match actual_decl {
            Decl::Defn {
                name,
                params,
                metadata,
                ..
            } => {
                let type_str = type_results
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, s)| format!("{}", s.ty))
                    .unwrap_or_else(|| "?".to_string());

                let param_infos: Vec<ParamInfo> = params
                    .iter()
                    .map(|p| {
                        let desc = metadata.as_ref().and_then(|m| {
                            m.params
                                .iter()
                                .find(|(n, _)| n == &p.name)
                                .map(|(_, d)| d.clone())
                        });
                        ParamInfo {
                            name: p.name.clone(),
                            ty: p
                                .ty
                                .as_ref()
                                .map(|t| format!("{t}"))
                                .unwrap_or_else(|| "?".to_string()),
                            description: desc,
                        }
                    })
                    .collect();

                functions.push(FunctionInfo {
                    name: name.clone(),
                    params: param_infos,
                    return_type: type_str,
                    doc: metadata.as_ref().and_then(|m| m.doc.clone()),
                    module: module_name.clone(),
                    is_private: is_priv || is_private_set.contains(name.as_str()),
                });
            }
            Decl::RecordDef { name, fields, .. } => {
                let field_infos: Vec<FieldInfo> = fields
                    .iter()
                    .map(|(fname, ftype)| FieldInfo {
                        name: fname.clone(),
                        ty: format!("{ftype}"),
                    })
                    .collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Record {
                        fields: field_infos,
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TypeDef { name, variants, .. } => {
                let variant_infos: Vec<VariantInfo> = variants
                    .iter()
                    .map(|v| VariantInfo {
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|f| format!("{f}")).collect(),
                    })
                    .collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Adt {
                        variants: variant_infos,
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TypeAlias { name, target, .. } => {
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Alias {
                        target: format!("{target}"),
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TraitDef { name, methods, .. } => {
                let method_names: Vec<String> = methods.iter().map(|m| m.name.clone()).collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Trait {
                        methods: method_names,
                    },
                    type_params: vec![],
                    doc: None,
                });
            }
            _ => {}
        }
    }

    // 依存関係
    let dependencies: Vec<DependencyInfo> = infer
        .module_env
        .imports
        .iter()
        .map(|imp| {
            let kind = if imp.open {
                DependencyKind::OpenImport
            } else if let Some(ref only) = imp.only {
                DependencyKind::SelectiveImport {
                    symbols: only.clone(),
                }
            } else {
                DependencyKind::Import
            };
            DependencyInfo {
                from: module_name.clone().unwrap_or_else(|| "main".to_string()),
                to: imp.module.clone(),
                kind,
            }
        })
        .collect();

    Knowledge {
        project: ProjectInfo {
            name: module_name.unwrap_or_else(|| "unnamed".to_string()),
            version: "0.1.0".to_string(),
        },
        functions,
        types,
        dependencies,
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct ImportVisibilitySpec {
    only: Option<Vec<String>>,
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct ResolvedImportModule {
    results: Vec<(String, lsharp_types::types::TypeScheme)>,
    hidden: std::collections::HashSet<String>,
}

#[cfg(test)]
fn collect_import_visibility(
    program: &lsharp_syntax::ast::Program,
) -> std::collections::HashMap<String, ImportVisibilitySpec> {
    let mut imports = std::collections::HashMap::new();
    for decl in &program.decls {
        if let lsharp_syntax::ast::Decl::ImportDecl { module, only, .. } = decl {
            let entry = imports
                .entry(module.clone())
                .or_insert_with(ImportVisibilitySpec::default);
            match (&mut entry.only, only.as_ref()) {
                (None, None) => {}
                (slot @ None, Some(next)) => {
                    *slot = Some(next.clone());
                }
                (Some(existing), Some(next)) => {
                    for symbol in next {
                        if !existing.contains(symbol) {
                            existing.push(symbol.clone());
                        }
                    }
                }
                (Some(_), None) => {
                    entry.only = None;
                }
            }
        }
    }
    imports
}

#[cfg(test)]
fn declared_module_name(
    program: &lsharp_syntax::ast::Program,
    fallback_file: &std::path::Path,
) -> String {
    program
        .decls
        .iter()
        .find_map(|decl| {
            if let lsharp_syntax::ast::Decl::ModuleDecl { name, .. } = decl {
                Some(name.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            fallback_file
                .file_stem()
                .and_then(|stem| stem.to_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "Main".to_string())
}

#[cfg(test)]
fn resolve_import_module_recursive(
    module: &str,
    from_module: &str,
    search_paths: &lsharp_ir::module_graph::ModuleSearchPaths,
    cache: &mut std::collections::HashMap<String, ResolvedImportModule>,
    resolving: &mut std::collections::HashSet<String>,
) -> miette::Result<ResolvedImportModule> {
    if let Some(cached) = cache.get(module) {
        return Ok(cached.clone());
    }
    if !resolving.insert(module.to_string()) {
        return Ok(cache.get(module).cloned().unwrap_or_default());
    }

    let result = (|| -> miette::Result<ResolvedImportModule> {
        let import_path = lsharp_ir::module_graph::ModuleGraph::resolve_module_import_path(
            module,
            from_module,
            search_paths,
        )
        .map_err(|e| miette::miette!("{e}"))?
        .ok_or_else(|| {
            miette::miette!(
                "モジュール '{}' が見つかりません ('{}' からインポート)",
                module,
                from_module
            )
        })?;

        let import_source = std::fs::read_to_string(&import_path)
            .map_err(|e| miette::miette!("{}: {}", import_path.display(), e))?;
        let import_program = lsharp_syntax::parse(&import_source)
            .map_err(|e| miette::miette!("{}: {e}", import_path.display()))?;
        let import_module_name = declared_module_name(&import_program, &import_path);

        let mut import_infer = lsharp_types::infer::Infer::new();
        for (dependency, spec) in collect_import_visibility(&import_program) {
            let dependency_surface = resolve_import_module_recursive(
                &dependency,
                &import_module_name,
                search_paths,
                cache,
                resolving,
            )?;
            import_infer.inject_external_types_for_import(
                &dependency,
                spec.only.as_deref(),
                &dependency_surface.hidden,
                &dependency_surface.results,
            );
        }

        let import_results = import_infer
            .infer_program(&import_program)
            .map_err(|e| miette::miette!("{}: {e}", import_path.display()))?;
        Ok(ResolvedImportModule {
            results: import_results,
            hidden: import_infer.module_env.privates.iter().cloned().collect(),
        })
    })();

    resolving.remove(module);
    if let Ok(surface) = &result {
        cache.insert(module.to_string(), surface.clone());
    }
    result
}

/// 型チェック内部用: import 宣言を再帰的に解決する
///
/// package root / src / .lsharp/packages / stdlib の探索順に従って import を解決し、
/// 各 import ごとに `:only` / `private` / package exports を反映した型環境だけを注入する。
#[cfg(test)]
fn resolve_imports_recursive(
    program: &lsharp_syntax::ast::Program,
    entry_file: &std::path::Path,
    infer: &mut lsharp_types::infer::Infer,
    resolved: &mut std::collections::HashSet<String>,
) -> miette::Result<()> {
    let search_paths = lsharp_ir::module_graph::ModuleSearchPaths::discover(entry_file);
    let current_module = declared_module_name(program, entry_file);
    let mut cache = std::collections::HashMap::new();

    for (module, spec) in collect_import_visibility(program) {
        let imported = resolve_import_module_recursive(
            &module,
            &current_module,
            &search_paths,
            &mut cache,
            resolved,
        )?;
        infer.inject_external_types_for_import(
            &module,
            spec.only.as_deref(),
            &imported.hidden,
            &imported.results,
        );
    }

    Ok(())
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

    fs::create_dir_all(&project_dir).map_err(|e| miette::miette!("ディレクトリ作成失敗: {e}"))?;

    // 標準ディレクトリ作成
    let src_dir = project_dir.join("src");
    let examples_dir = project_dir.join("examples");
    let tests_dir = project_dir.join("tests");
    let docs_dir = project_dir.join("docs");
    fs::create_dir_all(&src_dir).map_err(|e| miette::miette!("src ディレクトリ作成失敗: {e}"))?;
    fs::create_dir_all(&examples_dir)
        .map_err(|e| miette::miette!("examples ディレクトリ作成失敗: {e}"))?;
    fs::create_dir_all(&tests_dir)
        .map_err(|e| miette::miette!("tests ディレクトリ作成失敗: {e}"))?;
    fs::create_dir_all(&docs_dir).map_err(|e| miette::miette!("docs ディレクトリ作成失敗: {e}"))?;

    // lsharp.toml 生成
    let toml_content = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
entry = "src/Main.ls"
"#
    );
    fs::write(project_dir.join("lsharp.toml"), toml_content)
        .map_err(|e| miette::miette!("lsharp.toml 作成失敗: {e}"))?;

    // Main.ls 生成
    let main_content = r#"(module Main)

(defn main []
  (print 42))
"#;
    fs::write(src_dir.join("Main.ls"), main_content)
        .map_err(|e| miette::miette!("Main.ls 作成失敗: {e}"))?;

    // .gitignore 生成
    let gitignore_content = "*.wasm\n/target/\n/.lsharp/\n";
    fs::write(project_dir.join(".gitignore"), gitignore_content)
        .map_err(|e| miette::miette!(".gitignore 作成失敗: {e}"))?;

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
            .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
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
            .map_err(|e| miette::miette!("{}: {}", parent.display(), e))?;
    }
    std::fs::write(&output_path, json)
        .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
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
    let config = config::load_config_result(project_dir).map_err(|e| miette::miette!("{e}"))?;
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
        .map_err(|e| miette::miette!("{}: {}", docs_dir.display(), e))?;
    let api_path = docs_dir.join("api.json");
    std::fs::write(&api_path, &api_json)
        .map_err(|e| miette::miette!("{}: {}", api_path.display(), e))?;
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
    let content =
        std::fs::read_to_string(path).map_err(|e| miette::miette!("{}: {}", path.display(), e))?;
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
        .map_err(|e| miette::miette!("{}: {}", config_path.display(), e))?;
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
        .map_err(|e| miette::miette!("{}: {}", config_path.display(), e))?;
    println!("Added {package_name} to lsharp.toml");
    Ok(())
}

/// 指定ディレクトリを基点に依存パッケージをインストール (テスト用に分離)
fn cmd_install_in(project_dir: &Path) -> miette::Result<()> {
    let config = config::load_config_result(project_dir).map_err(|e| miette::miette!("{e}"))?;

    let deps = &config.dependencies;

    let lsharp_dir = project_dir.join(".lsharp");
    let packages_dir = lsharp_dir.join("packages");
    std::fs::create_dir_all(&packages_dir)
        .map_err(|e| miette::miette!(".lsharp/packages/ の作成に失敗: {e}"))?;

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

                let abs_resolved = resolved.canonicalize().map_err(|e| {
                    miette::miette!("パスの正規化に失敗 '{}': {e}", resolved.display())
                })?;
                let source_id = dependency_source_string(spec, project_dir);
                let link_path = installed_package_dir(&packages_dir, name, &source_id);
                // 既存のシンボリックリンクがあれば削除
                if link_path.exists() || link_path.symlink_metadata().is_ok() {
                    std::fs::remove_file(&link_path)
                        .or_else(|_| std::fs::remove_dir_all(&link_path))
                        .map_err(|e| miette::miette!("既存リンクの削除に失敗: {e}"))?;
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&abs_resolved, &link_path)
                    .map_err(|e| miette::miette!("シンボリックリンク作成に失敗 '{name}': {e}"))?;

                #[cfg(not(unix))]
                std::fs::copy(&abs_resolved, &link_path)
                    .map_err(|e| miette::miette!("依存コピーに失敗 '{name}': {e}"))?;

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
        .map_err(|e| miette::miette!("{}: {}", lsharp_dir.display(), e))?;
    let lock_path = lsharp_dir.join("lock.toml");
    lockfile::write_lockfile(&lock, &lock_path).map_err(|e| miette::miette!("{e}"))?;
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
        .map_err(|e| miette::miette!("{}: {}", docs_dir.display(), e))?;
    let output_path = docs_dir.join("api.json");
    std::fs::write(&output_path, json)
        .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;
    Ok(Some(output_path))
}

fn rebuild_installed_module_index(project_dir: &Path) -> miette::Result<()> {
    let index_root = project_dir.join(".lsharp").join("module-index");
    if index_root.exists() {
        std::fs::remove_dir_all(&index_root)
            .map_err(|e| miette::miette!("{}: {}", index_root.display(), e))?;
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
                .map_err(|e| miette::miette!("{}: {}", parent.display(), e))?;
        }

        let target = file
            .strip_prefix(project_dir)
            .unwrap_or(file.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        std::fs::write(&index_path, target)
            .map_err(|e| miette::miette!("{}: {}", index_path.display(), e))?;
    }

    Ok(())
}

fn collect_package_source_files(dir: &Path, out: &mut Vec<PathBuf>) -> miette::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let entries =
        std::fs::read_dir(dir).map_err(|e| miette::miette!("{}: {}", dir.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| miette::miette!("{}: {}", dir.display(), e))?;
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

/// git clone コマンドの引数を構築する (テスト用)
#[cfg(test)]
fn build_git_clone_args<'a>(
    url: &'a str,
    branch: Option<&'a str>,
    tag: Option<&'a str>,
    dest: &'a str,
) -> Vec<&'a str> {
    let mut args = vec!["clone", "--depth", "1"];
    let ref_spec = branch.or(tag);
    if let Some(r) = ref_spec {
        args.push("--branch");
        args.push(r);
    }
    args.push(url);
    args.push(dest);
    args
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_guest_compile_success_does_not_request_host_fallback() {
        assert!(!should_fallback_to_host_compile(Some(0)));
        assert!(should_fallback_to_host_compile(Some(1)));
        assert!(should_fallback_to_host_compile(None));
    }

    #[test]
    fn test_test_command_is_selfhost_shadow_command() {
        assert!(is_selfhost_shadow_command("test"));
        assert!(!is_selfhost_shadow_command("compile"));
    }

    fn command_names_from_help(help: &str) -> Vec<&str> {
        help.lines()
            .filter_map(|line| line.strip_prefix("  "))
            .filter_map(|line| {
                let name = line.split_whitespace().next()?;
                (!name.starts_with('-')).then_some(name)
            })
            .collect()
    }

    fn os_args(args: &[&str]) -> Vec<std::ffi::OsString> {
        args.iter().map(std::ffi::OsString::from).collect()
    }

    fn dot_prefixed(path: &str) -> String {
        Path::new(".").join(path).to_string_lossy().into_owned()
    }

    #[test]
    fn test_cli_help_excludes_removed_parse_check_fmt_subcommands() {
        let help = Cli::command().render_long_help().to_string();
        let commands = command_names_from_help(&help);

        assert!(commands.contains(&"compile"));
        assert!(commands.contains(&"language-guide"));
        assert!(!commands.contains(&"parse"));
        assert!(!commands.contains(&"check"));
        assert!(!commands.contains(&"fmt"));
    }

    #[test]
    fn test_cli_try_parse_from_rejects_removed_parse_check_fmt_subcommands() {
        for subcommand in ["parse", "check", "fmt"] {
            let err = match Cli::try_parse_from(["lsharp", subcommand, "examples/fib.ls"]) {
                Ok(_) => panic!("旧 CLI サブコマンドは拒否されるべき: {subcommand}"),
                Err(err) => err,
            };

            assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
            assert!(err.to_string().contains(subcommand));
        }
    }

    #[test]
    fn test_cli_compile_target_accepts_wasi_component_alias_and_web_wasm() {
        let cli = Cli::try_parse_from([
            "lsharp",
            "compile",
            "examples/fib.ls",
            "--target",
            "wasi-component",
        ])
        .expect("wasi-component target should parse");
        let Command::Compile { target, .. } = cli.command else {
            panic!("compile subcommand should parse");
        };
        assert_eq!(target, Some(CliCompileTarget::WasiComponent));

        let cli = Cli::try_parse_from([
            "lsharp",
            "compile",
            "examples/fib.ls",
            "--target",
            "wasi-preview1",
        ])
        .expect("wasi-preview1 target should parse");
        let Command::Compile { target, .. } = cli.command else {
            panic!("compile subcommand should parse");
        };
        assert_eq!(target, Some(CliCompileTarget::WasiPreview1));

        let cli = Cli::try_parse_from(["lsharp", "compile", "examples/fib.ls", "--target", "wasm"])
            .expect("wasm alias should parse");
        let Command::Compile { target, .. } = cli.command else {
            panic!("compile subcommand should parse");
        };
        assert_eq!(target, Some(CliCompileTarget::WasiComponent));

        let cli = Cli::try_parse_from([
            "lsharp",
            "compile",
            "examples/fib.ls",
            "--target",
            "web-wasm",
        ])
        .expect("web-wasm target should parse");
        let Command::Compile { target, .. } = cli.command else {
            panic!("compile subcommand should parse");
        };
        assert_eq!(target, Some(CliCompileTarget::WebWasm));
    }

    #[test]
    fn test_cli_compile_backend_accepts_wasmgc_with_web_wasm_target() {
        let cli = Cli::try_parse_from([
            "lsharp",
            "compile",
            "examples/fib.ls",
            "--backend",
            "wasmgc",
            "--target",
            "web-wasm",
        ])
        .expect("wasmgc backend should parse");
        let Command::Compile {
            backend, target, ..
        } = cli.command
        else {
            panic!("compile subcommand should parse");
        };
        assert_eq!(backend, Some(CliCompileBackend::WasmGc));
        assert_eq!(target, Some(CliCompileTarget::WebWasm));
    }

    #[test]
    fn test_should_delegate_to_embedded_component_args_accepts_compile_build_component_subset() {
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "compile",
            "examples/fib.ls",
            "-o",
            "fib.component.wasm",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "build",
            "examples/fib.ls",
            "--target",
            "wasi-preview1",
            "--output",
            "fib.wasm",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "review",
            "examples/fib.ls",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "review",
            "examples/fib.ls",
            "--json",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "review",
            "examples/fib.ls",
            "--format",
            "json",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "doc-ack",
            "examples/fib.ls",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "doc-ack",
            "examples/fib.ls",
            "--trailer",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "doc-check",
            "examples/fib.ls",
        ])));
        assert!(should_delegate_to_embedded_component_args(&os_args(&[
            "doc-check",
            "examples/fib.ls",
            "--strict",
        ])));
    }

    #[test]
    fn test_should_delegate_to_embedded_component_args_rejects_rust_only_compile_build_flags() {
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "compile", "--help",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "compile",
            "examples/fib.ls",
            "--emit-ir",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "compile",
            "examples/fib.ls",
            "--target",
            "web-wasm",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "compile",
            "examples/fib.ls",
            "--backend",
            "wasmgc",
            "--target",
            "web-wasm",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "build",
            "examples/fib.ls",
            "--output",
            "--target",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "review", "--help",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "review",
            "examples/fib.ls",
            "--format",
            "yaml",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "review",
            "examples/fib.ls",
            "--json",
            "--format",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "doc-ack", "--help",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "doc-check",
            "--help",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "doc-ack",
            "examples/fib.ls",
            "--json",
        ])));
        assert!(!should_delegate_to_embedded_component_args(&os_args(&[
            "doc-check",
            "examples/fib.ls",
            "--format",
            "json",
        ])));
    }

    #[test]
    fn test_normalize_guest_args_prefixes_relative_src_entry_paths() {
        let current_dir = std::env::temp_dir().join("lsharp_normalize_guest_args");
        let src_entry = dot_prefixed("src/Main.ls");

        let compile_args = normalize_guest_args_for_current_dir(
            &current_dir,
            vec!["compile".to_string(), "src/Main.ls".to_string()],
        );
        assert_eq!(compile_args, vec!["compile".to_string(), src_entry.clone()]);

        let review_args = normalize_guest_args_for_current_dir(
            &current_dir,
            vec!["review".to_string(), "src/Main.ls".to_string()],
        );
        assert_eq!(review_args, vec!["review".to_string(), src_entry]);
    }

    #[test]
    fn test_normalize_guest_args_relativizes_absolute_src_entry_paths_with_dot_prefix() {
        let current_dir = std::env::temp_dir().join("lsharp_normalize_guest_args_abs");
        let src_entry = current_dir.join("src/Main.ls");
        let expected = dot_prefixed("src/Main.ls");

        let compile_args = normalize_guest_args_for_current_dir(
            &current_dir,
            vec![
                "compile".to_string(),
                src_entry.to_string_lossy().into_owned(),
                "--output".to_string(),
                current_dir
                    .join("src/Main.component.wasm")
                    .to_string_lossy()
                    .into_owned(),
            ],
        );

        assert_eq!(compile_args[1], expected);
        assert_eq!(compile_args[3], dot_prefixed("src/Main.component.wasm"));
    }

    #[test]
    fn test_normalize_guest_args_keeps_non_src_relative_paths_unchanged() {
        let current_dir = std::env::temp_dir().join("lsharp_normalize_guest_args_examples");
        let compile_args = normalize_guest_args_for_current_dir(
            &current_dir,
            vec!["compile".to_string(), "examples/fib.ls".to_string()],
        );

        assert_eq!(
            compile_args,
            vec!["compile".to_string(), "examples/fib.ls".to_string()]
        );
    }

    #[test]
    fn test_cmd_install_no_dependencies() {
        // lsharp.toml がないディレクトリではデフォルト設定 (依存なし) で成功する
        let dir = std::env::temp_dir().join("lsharp_test_install_no_deps");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let result = cmd_install_in(&dir);
        assert!(result.is_ok(), "依存なしで cmd_install_in は成功するべき");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_install_missing_toml_uses_defaults() {
        // lsharp.toml が存在しないディレクトリでもデフォルト設定で動作する
        let dir = std::env::temp_dir().join("lsharp_test_install_missing_toml");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        // lsharp.toml を作成しない
        let result = cmd_install_in(&dir);
        assert!(
            result.is_ok(),
            "lsharp.toml がなくてもデフォルトで成功するべき"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_doc_json_writes_docs_api_json() {
        let dir = std::env::temp_dir().join("lsharp_test_doc_json");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("lsharp.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.2.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/Geometry.ls"),
            "(module Geometry)\n(defn add [x y] (+ x y))",
        )
        .unwrap();

        let result = cmd_doc_json(&dir.join("src/Geometry.ls"), None);
        assert!(result.is_ok(), "doc --json は成功するべき: {result:?}");

        let api_path = dir.join("docs").join("api.json");
        let content = std::fs::read_to_string(&api_path).unwrap();
        assert!(content.contains("\"package\": \"demo\""));
        assert!(content.contains("\"version\": \"0.2.0\""));
        assert!(content.contains("\"name\": \"Geometry\""));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_test_succeeds_for_metadata_fixture() {
        let dir = std::env::temp_dir().join("lsharp_test_metadata_command");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("metadata.ls");
        std::fs::write(
            &file,
            r#"(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"#,
        )
        .unwrap();

        let result = cmd_test(&file);
        assert!(
            result.is_ok(),
            "metadata test command should succeed: {result:?}"
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_repo_doc_status_dogfooding_is_wired_for_metadata_fixture() {
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let status_path = repo_root.join(".lsharp-doc-status");
        assert!(
            status_path.exists(),
            ".lsharp-doc-status を repo で運用するべき"
        );

        let status = lsharp_docs::tracker::load_doc_status(&status_path);
        let abs = status
            .entries
            .get("abs")
            .expect("examples/metadata.ls の abs は doc-status で追跡するべき");
        assert_eq!(abs.freshness, lsharp_docs::tracker::Freshness::Fresh);
        assert_eq!(abs.reviewed_by.as_deref(), Some("docs-maintainers"));
        assert!(abs.last_reviewed.is_some(), "初回 ack の日時を保持するべき");

        let ci = std::fs::read_to_string(repo_root.join(".github/workflows/ci.yml")).unwrap();
        assert!(
            ci.contains("scripts/ci/doc-status-check.sh"),
            "CI は doc-status check script を実行するべき"
        );

        let operation_doc =
            repo_root.join("docs/development/operations/documentation-freshness.md");
        assert!(operation_doc.exists(), "doc-status 運用手順が必要");

        let site_manifest = std::fs::read_to_string(repo_root.join("docs/site.toml")).unwrap();
        assert!(
            site_manifest.contains("docs/development/operations/documentation-freshness.md"),
            "doc-status 運用手順は docs site に公開するべき"
        );
    }

    #[test]
    fn test_has_metadata_errors_detects_lowercase_error_diagnostics() {
        let diagnostics = vec![
            "[warning] add: doc note".to_string(),
            "[error] abs: unknown-fn in :invariant".to_string(),
        ];

        assert!(
            has_metadata_errors(&diagnostics),
            "metadata diagnostics は lowercase display でも error を検出するべき"
        );
    }

    #[test]
    fn test_cmd_check_package_generates_api_json_and_checksum() {
        let dir = std::env::temp_dir().join("lsharp_test_check_package");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("lsharp.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.3.0\"\nentry = \"src/Geometry.ls\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/Geometry.ls"),
            "(module Geometry)\n(defn add [x y] :doc \"加算\" (+ x y))",
        )
        .unwrap();

        let summary = cmd_check_package_in(&dir, None, None).unwrap();

        assert!(summary.contains("Validating lsharp.toml ... ok"));
        assert!(summary.contains("Generating api.json ... ok"));
        assert!(summary.contains("checksum: sha256:"));
        assert!(dir.join("docs/api.json").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_info_reads_installed_package_api() {
        let dir = std::env::temp_dir().join("lsharp_test_info_package");
        let _ = std::fs::remove_dir_all(&dir);
        let package_dir = dir.join(".lsharp/packages/mylib-12345678");
        std::fs::create_dir_all(package_dir.join("docs")).unwrap();
        std::fs::create_dir_all(dir.join(".lsharp")).unwrap();
        std::fs::write(
            dir.join(".lsharp/lock.toml"),
            r#"
[[package]]
name = "mylib"
version = "0.2.0"
source = "git:https://github.com/user/mylib.git?tag=v0.2.0"
"#,
        )
        .unwrap();
        std::fs::write(
            package_dir.join("docs/api.json"),
            r#"{
  "package": "mylib",
  "version": "0.2.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [
        {
          "name": "distance",
          "signature": "Point -> Point -> Float",
          "params": [],
          "returns": { "type": "Float", "doc": null },
          "doc": "2 点間の距離",
          "example": null
        }
      ],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();

        let summary = cmd_info_in(&dir, "mylib").unwrap();

        assert!(summary.contains("Package: mylib@0.2.0"));
        assert!(summary.contains("Source: git:https://github.com/user/mylib.git?tag=v0.2.0"));
        assert!(summary.contains("Geometry.distance : Point -> Point -> Float - 2 点間の距離"));
        assert!(summary.contains("Types:\n  (none)\n"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_api_diff_reports_added_changed_removed() {
        let dir = std::env::temp_dir().join("lsharp_test_api_diff");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let old = dir.join("old.json");
        let new = dir.join("new.json");
        std::fs::write(
            &old,
            r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [
        {
          "name": "distance",
          "signature": "Point -> Point -> Int",
          "params": [],
          "returns": { "type": "Int", "doc": null },
          "doc": null,
          "example": null
        },
        {
          "name": "obsolete",
          "signature": "Int -> Int",
          "params": [],
          "returns": { "type": "Int", "doc": null },
          "doc": null,
          "example": null
        }
      ],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();
        std::fs::write(
            &new,
            r#"{
  "package": "demo",
  "version": "0.2.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [
        {
          "name": "distance",
          "signature": "Point -> Point -> Float",
          "params": [],
          "returns": { "type": "Float", "doc": null },
          "doc": null,
          "example": null
        },
        {
          "name": "rotate",
          "signature": "Vec2 -> Float -> Vec2",
          "params": [],
          "returns": { "type": "Vec2", "doc": null },
          "doc": null,
          "example": null
        }
      ],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();

        let summary =
            cmd_api_diff_specs(&dir, &old.display().to_string(), &new.display().to_string())
                .unwrap();

        assert!(summary.contains("Added:    + Geometry.rotate : Vec2 -> Float -> Vec2"));
        assert!(summary.contains("Changed:  ~ Geometry.distance : Point -> Point -> Int -> Point -> Point -> Float  BREAKING"));
        assert!(summary.contains("Removed:  - Geometry.obsolete"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_api_diff_specs_supports_git_tags() {
        let dir = std::env::temp_dir().join("lsharp_test_api_diff_git_tags");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs")).unwrap();

        init_test_git_repo(&dir);
        std::fs::write(
            dir.join("docs/api.json"),
            r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();
        git_commit_all(&dir, "v0.1.0");
        git_tag(&dir, "v0.1.0");

        std::fs::write(
            dir.join("docs/api.json"),
            r#"{
  "package": "demo",
  "version": "0.2.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [
        {
          "name": "rotate",
          "signature": "Vec2 -> Float -> Vec2",
          "params": [],
          "returns": { "type": "Vec2", "doc": null },
          "doc": null,
          "example": null
        }
      ],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();
        git_commit_all(&dir, "v0.2.0");
        git_tag(&dir, "v0.2.0");

        let summary = cmd_api_diff_specs(&dir, "v0.1.0", "v0.2.0").unwrap();

        assert!(summary.contains("Added:    + Geometry.rotate : Vec2 -> Float -> Vec2"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_cmd_check_package_previous_tag_compares_against_git_tag() {
        let dir = std::env::temp_dir().join("lsharp_test_check_package_previous_tag");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("docs")).unwrap();

        init_test_git_repo(&dir);
        std::fs::write(
            dir.join("lsharp.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/Geometry.ls\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/Geometry.ls"),
            "(module Geometry)\n(defn distance [p1 p2] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("docs/api.json"),
            r#"{
  "package": "demo",
  "version": "0.1.0",
  "modules": [
    {
      "name": "Geometry",
      "doc": null,
      "functions": [
        {
          "name": "distance",
          "signature": "Point -> Point -> Int",
          "params": [],
          "returns": { "type": "Int", "doc": null },
          "doc": null,
          "example": null
        }
      ],
      "types": []
    }
  ]
}"#,
        )
        .unwrap();
        git_commit_all(&dir, "baseline");
        git_tag(&dir, "v0.1.0");

        std::fs::write(
            dir.join("src/Geometry.ls"),
            "(module Geometry)\n(defn distance [p1 p2] 1.0)\n(defn rotate [v angle] v)",
        )
        .unwrap();

        let summary = cmd_check_package_in(&dir, None, Some("v0.1.0")).unwrap();

        assert!(summary.contains("Comparing with v0.1.0 ..."));
        assert!(summary.contains("+ Geometry.rotate"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    fn init_test_git_repo(dir: &Path) {
        let output = std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "git init failed: {:?}", output);
    }

    fn git_commit_all(dir: &Path, message: &str) {
        let add = std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(add.status.success(), "git add failed: {:?}", add);

        let commit = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=Codex",
                "-c",
                "user.email=codex@example.com",
                "commit",
                "-m",
                message,
            ])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(commit.status.success(), "git commit failed: {:?}", commit);
    }

    fn git_tag(dir: &Path, tag: &str) {
        let output = std::process::Command::new("git")
            .args(["tag", tag])
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "git tag failed: {:?}", output);
    }

    #[test]
    fn test_cmd_install_path_dependency() {
        // Path 依存のインストールをテスト
        let base_dir = std::env::temp_dir().join("lsharp_test_install_path_dep");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        // 依存先ディレクトリを作成 (lsharp.toml を含む)
        let dep_dir = base_dir.join("mylib");
        std::fs::create_dir_all(dep_dir.join("src")).unwrap();
        std::fs::write(dep_dir.join("lsharp.toml"), "[project]\nname = \"mylib\"\n").unwrap();
        std::fs::write(
            dep_dir.join("src/Lib.ls"),
            "(module Lib)\n(defn helper [] 1)",
        )
        .unwrap();

        // プロジェクトの lsharp.toml を作成
        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies.mylib]\npath = \"mylib\"\n",
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(
            result.is_ok(),
            "Path 依存のインストールは成功するべき: {:?}",
            result
        );

        let link_path = find_installed_package_dir(&base_dir, "mylib")
            .expect(".lsharp/packages/<name>-<hash> が必要");
        assert!(
            link_path.exists(),
            "インストール済み package dir が存在するべき"
        );
        assert!(
            dep_dir.join("docs/api.json").exists(),
            "install 時に docs/api.json を生成するべき"
        );
        assert!(
            base_dir.join(".lsharp").join("lock.toml").exists(),
            "install 時に .lsharp/lock.toml を生成するべき"
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_install_path_dependency_writes_module_index_for_exported_modules() {
        let base_dir = std::env::temp_dir().join("lsharp_test_install_module_index");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        let dep_dir = base_dir.join("mylib");
        std::fs::create_dir_all(dep_dir.join("src/Geometry")).unwrap();
        std::fs::write(
            dep_dir.join("lsharp.toml"),
            "[project]\nname = \"mylib\"\n[project.exports]\nmodules = [\"Geometry\", \"Geometry.Vec2\"]\n",
        )
        .unwrap();
        std::fs::write(
            dep_dir.join("src/Geometry.ls"),
            "(module Geometry)\n(defn distance [] 1)",
        )
        .unwrap();
        std::fs::write(
            dep_dir.join("src/Geometry/Vec2.ls"),
            "(module Geometry.Vec2)\n(defn zero [] 0)",
        )
        .unwrap();
        std::fs::write(
            dep_dir.join("src/Hidden.ls"),
            "(module Hidden)\n(defn secret [] 99)",
        )
        .unwrap();

        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies.mylib]\npath = \"mylib\"\n",
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(result.is_ok(), "install は成功するべき: {result:?}");

        let installed_dir = find_installed_package_dir(&base_dir, "mylib")
            .expect(".lsharp/packages/<name>-<hash> が必要");
        let index_dir = base_dir.join(".lsharp/module-index");
        let geometry_index = index_dir.join("Geometry.path");
        let vec2_index = index_dir.join("Geometry/Vec2.path");
        let hidden_index = index_dir.join("Hidden.path");

        assert!(
            geometry_index.exists(),
            "exported module の index が生成されるべき"
        );
        assert!(
            vec2_index.exists(),
            "nested exported module の index が生成されるべき"
        );
        assert!(
            !hidden_index.exists(),
            "非公開 module の index は生成しないべき"
        );

        let geometry_target = std::fs::read_to_string(&geometry_index).unwrap();
        let vec2_target = std::fs::read_to_string(&vec2_index).unwrap();
        let installed_relative = installed_dir.strip_prefix(&base_dir).unwrap();
        assert_eq!(
            geometry_target.trim(),
            installed_relative
                .join("src/Geometry.ls")
                .display()
                .to_string()
        );
        assert_eq!(
            vec2_target.trim(),
            installed_relative
                .join("src/Geometry/Vec2.ls")
                .display()
                .to_string()
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_init_creates_standard_package_layout() {
        let base_dir = std::env::temp_dir().join("lsharp_test_init_layout");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        let result = cmd_init_in(&base_dir, "demo-lib");
        assert!(result.is_ok(), "init は成功するべき: {result:?}");

        let project_dir = base_dir.join("demo-lib");
        assert!(project_dir.join("lsharp.toml").exists());
        assert!(project_dir.join("src/Main.ls").exists());
        assert!(project_dir.join("examples").is_dir());
        assert!(project_dir.join("tests").is_dir());
        assert!(project_dir.join("docs").is_dir());
        assert!(project_dir.join(".gitignore").exists());

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_init_writes_main_entry_and_gitignore_defaults() {
        let base_dir = std::env::temp_dir().join("lsharp_test_init_contents");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        let result = cmd_init_in(&base_dir, "demo-app");
        assert!(result.is_ok(), "init は成功するべき: {result:?}");

        let project_dir = base_dir.join("demo-app");
        let toml = std::fs::read_to_string(project_dir.join("lsharp.toml")).unwrap();
        let main = std::fs::read_to_string(project_dir.join("src/Main.ls")).unwrap();
        let gitignore = std::fs::read_to_string(project_dir.join(".gitignore")).unwrap();

        assert!(toml.contains("entry = \"src/Main.ls\""));
        assert!(main.contains("(module Main)"));
        assert!(gitignore.contains("/.lsharp/"));

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_add_writes_tagged_github_dependency_to_lsharp_toml() {
        let base_dir = std::env::temp_dir().join("lsharp_test_add_dependency");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nentry = \"src/Main.ls\"\n",
        )
        .unwrap();

        let result = cmd_add_in(&base_dir, "github.com/user/geometry-utils", Some("v0.2.0"));
        assert!(result.is_ok(), "add は成功するべき: {result:?}");

        let content = std::fs::read_to_string(base_dir.join("lsharp.toml")).unwrap();
        assert!(content.contains("[dependencies.geometry-utils]"));
        assert!(content.contains("git = \"https://github.com/user/geometry-utils.git\""));
        assert!(content.contains("tag = \"v0.2.0\""));

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_add_rejects_duplicate_dependency_name() {
        let base_dir = std::env::temp_dir().join("lsharp_test_add_dependency_duplicate");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();
        std::fs::write(
            base_dir.join("lsharp.toml"),
            r#"[project]
name = "demo"
version = "0.1.0"
entry = "src/Main.ls"

[dependencies.geometry-utils]
git = "https://github.com/user/geometry-utils.git"
tag = "v0.1.0"
"#,
        )
        .unwrap();

        let result = cmd_add_in(
            &base_dir,
            "https://github.com/user/geometry-utils",
            Some("v0.2.0"),
        );
        assert!(result.is_err(), "重複 dependency は失敗するべき");

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_install_path_dependency_missing_path() {
        // 存在しないパス依存はスキップされる (エラーにはならない)
        let base_dir = std::env::temp_dir().join("lsharp_test_install_missing_path");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies.missing]\npath = \"nonexistent\"\n",
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(result.is_ok(), "存在しないパスでもエラーにはならないべき");

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_check_import_open_polymorphic_helper_stays_generalized() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_test_check_import_poly_helper_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(defn choose-first [x y] x)\n(defn helper [] 0)",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils :open)\n(defn main [] (do (print (choose-first 1 true)) (if (choose-first true 1) (print 1) (print 0))))",
        )
        .unwrap();

        let source = std::fs::read_to_string(dir.join("Main.ls")).unwrap();
        let program = lsharp_syntax::parse(&source).unwrap();
        let mut infer = lsharp_types::infer::Infer::new();
        let mut resolved_modules = std::collections::HashSet::new();

        resolve_imports_recursive(
            &program,
            &dir.join("Main.ls"),
            &mut infer,
            &mut resolved_modules,
        )
        .unwrap();
        let results = infer.infer_program(&program);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            results.is_ok(),
            "extra helper があっても open import の多相関数は一般化を保つべき: {:?}",
            results.err()
        );
    }

    #[test]
    fn test_check_selfhost_typeinfer_standalone_import_path() {
        let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let file = project_root.join("selfhost/src/Types/TypeInfer.ls");
        let source = std::fs::read_to_string(&file).unwrap();
        let program = lsharp_syntax::parse(&source).unwrap();
        let mut infer = lsharp_types::infer::Infer::new();
        let mut resolved_modules = std::collections::HashSet::new();

        resolve_imports_recursive(&program, &file, &mut infer, &mut resolved_modules).unwrap();
        let results = infer.infer_program(&program);

        assert!(
            results.is_ok(),
            "selfhost/src/Types/TypeInfer.ls standalone check path は成功するべき: {:?}",
            results.err()
        );
    }

    #[test]
    fn test_check_import_only_blocks_non_selected_symbol() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_test_check_import_only_blocks_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(defn helper [] 1)\n(defn secret [] 2)",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils :only [helper])\n(defn main [] (secret))",
        )
        .unwrap();

        let source = std::fs::read_to_string(dir.join("Main.ls")).unwrap();
        let program = lsharp_syntax::parse(&source).unwrap();
        let mut infer = lsharp_types::infer::Infer::new();
        let mut resolved_modules = std::collections::HashSet::new();

        resolve_imports_recursive(
            &program,
            &dir.join("Main.ls"),
            &mut infer,
            &mut resolved_modules,
        )
        .unwrap();
        let results = infer.infer_program(&program);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            results.is_err(),
            ":only で除外されたシンボルは参照できないべき"
        );
    }

    #[test]
    fn test_check_private_import_blocks_symbol() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_test_check_private_import_blocks_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(private (defn secret [] 2))\n(defn helper [] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("Main.ls"),
            "(module Main)\n(import Utils)\n(defn main [] (secret))",
        )
        .unwrap();

        let source = std::fs::read_to_string(dir.join("Main.ls")).unwrap();
        let program = lsharp_syntax::parse(&source).unwrap();
        let mut infer = lsharp_types::infer::Infer::new();
        let mut resolved_modules = std::collections::HashSet::new();

        resolve_imports_recursive(
            &program,
            &dir.join("Main.ls"),
            &mut infer,
            &mut resolved_modules,
        )
        .unwrap();
        let results = infer.infer_program(&program);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            results.is_err(),
            "private なシンボルは他モジュールから参照できないべき"
        );
    }

    #[test]
    fn test_check_resolves_packages_from_project_root() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_test_check_project_root_packages_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("examples/demo")).unwrap();
        std::fs::create_dir_all(dir.join(".lsharp/packages/pkg-123/src")).unwrap();
        std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"demo\"\n").unwrap();
        std::fs::write(
            dir.join(".lsharp/packages/pkg-123/src/Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("examples/demo/Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] (helper))",
        )
        .unwrap();

        let main_file = dir.join("examples/demo/Main.ls");
        let source = std::fs::read_to_string(&main_file).unwrap();
        let program = lsharp_syntax::parse(&source).unwrap();
        let mut infer = lsharp_types::infer::Infer::new();
        let mut resolved_modules = std::collections::HashSet::new();

        resolve_imports_recursive(&program, &main_file, &mut infer, &mut resolved_modules).unwrap();
        let results = infer.infer_program(&program);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            results.is_ok(),
            "project root の packages 配下を探索できるべき"
        );
    }

    #[test]
    fn test_check_rejects_non_exported_package_module() {
        let dir = std::env::temp_dir().join(format!(
            "lsharp_test_check_package_exports_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".lsharp/packages/demo-123/src")).unwrap();
        std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"app\"\n").unwrap();
        std::fs::write(
            dir.join(".lsharp/packages/demo-123/lsharp.toml"),
            "[project]\nname=\"demo\"\n[project.exports]\nmodules=[\"Public\"]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(".lsharp/packages/demo-123/src/Hidden.ls"),
            "(module Hidden)\n(defn helper [] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/Main.ls"),
            "(module Main)\n(import Hidden)\n(defn main [] 0)",
        )
        .unwrap();

        let main_file = dir.join("src/Main.ls");
        let source = std::fs::read_to_string(&main_file).unwrap();
        let program = lsharp_syntax::parse(&source).unwrap();
        let mut infer = lsharp_types::infer::Infer::new();
        let mut resolved_modules = std::collections::HashSet::new();

        let result =
            resolve_imports_recursive(&program, &main_file, &mut infer, &mut resolved_modules);
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            result.is_err(),
            "非公開 package module の import は失敗するべき"
        );
    }

    #[test]
    fn test_build_git_clone_args_basic() {
        // branch/tag なしの場合
        let args = build_git_clone_args(
            "https://github.com/user/repo.git",
            None,
            None,
            ".lsharp/packages/repo-12345678",
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "--depth",
                "1",
                "https://github.com/user/repo.git",
                ".lsharp/packages/repo-12345678",
            ]
        );
    }

    #[test]
    fn test_build_git_clone_args_with_branch() {
        let args = build_git_clone_args(
            "https://github.com/user/repo.git",
            Some("develop"),
            None,
            ".lsharp/packages/repo-12345678",
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "--depth",
                "1",
                "--branch",
                "develop",
                "https://github.com/user/repo.git",
                ".lsharp/packages/repo-12345678",
            ]
        );
    }

    #[test]
    fn test_build_git_clone_args_with_tag() {
        let args = build_git_clone_args(
            "https://github.com/user/repo.git",
            None,
            Some("v1.0.0"),
            ".lsharp/packages/repo-12345678",
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "--depth",
                "1",
                "--branch",
                "v1.0.0",
                "https://github.com/user/repo.git",
                ".lsharp/packages/repo-12345678",
            ]
        );
    }

    #[test]
    fn test_build_git_clone_args_branch_takes_priority_over_tag() {
        // branch と tag の両方が指定された場合、branch が優先される
        let args = build_git_clone_args(
            "https://github.com/user/repo.git",
            Some("main"),
            Some("v1.0.0"),
            ".lsharp/packages/repo-12345678",
        );
        assert_eq!(
            args,
            vec![
                "clone",
                "--depth",
                "1",
                "--branch",
                "main",
                "https://github.com/user/repo.git",
                ".lsharp/packages/repo-12345678",
            ]
        );
    }

    #[test]
    fn test_git_clone_invalid_url_returns_error() {
        // 存在しない URL でクローンするとエラーを返す (クラッシュしない)
        let dir = std::env::temp_dir().join("lsharp_test_git_clone_invalid");
        let _ = std::fs::remove_dir_all(&dir);

        let dest = dir.join("nonexistent-repo");
        let result = git_clone(
            "https://invalid.example.com/no-such-repo.git",
            None,
            None,
            &dest,
        );

        assert!(
            result.is_err(),
            "存在しない URL の git clone はエラーを返すべき"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("git clone 失敗") || err_msg.contains("git コマンドの実行に失敗"),
            "エラーメッセージに適切な情報が含まれるべき: {err_msg}"
        );

        // クローン先ディレクトリが残っていれば削除
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_cmd_install_git_dependency_already_exists() {
        // 既にクローン済みのディレクトリがある場合はスキップされる
        let base_dir = std::env::temp_dir().join("lsharp_test_install_git_exists");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        // 依存先ディレクトリを手動で作成 (クローン済みを模擬)
        let source_id = dependency_source_string(
            &config::DependencySpec::Git {
                git: "https://github.com/user/mylib.git".to_string(),
                branch: Some("main".to_string()),
                tag: None,
            },
            &base_dir,
        );
        let deps_dir = installed_package_dir(
            &base_dir.join(".lsharp").join("packages"),
            "mylib",
            &source_id,
        );
        std::fs::create_dir_all(&deps_dir).unwrap();

        std::fs::write(
            base_dir.join("lsharp.toml"),
            r#"[dependencies.mylib]
git = "https://github.com/user/mylib.git"
branch = "main"
"#,
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(
            result.is_ok(),
            "既存ディレクトリがあればスキップして成功するべき"
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_install_git_dependency_clone_failure() {
        // 無効な URL での git clone はエラーメッセージを出してスキップする (全体はエラーにならない)
        let base_dir = std::env::temp_dir().join("lsharp_test_install_git_fail");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        std::fs::write(
            base_dir.join("lsharp.toml"),
            r#"[dependencies.badrepo]
git = "https://invalid.example.com/no-such-repo.git"
"#,
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(
            result.is_ok(),
            "git clone 失敗でも全体はエラーにならないべき"
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_install_path_dependency_no_toml() {
        // lsharp.toml がない依存先はスキップされる
        let base_dir = std::env::temp_dir().join("lsharp_test_install_no_dep_toml");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        // 依存先ディレクトリを作成するが lsharp.toml は配置しない
        let dep_dir = base_dir.join("noconfig");
        std::fs::create_dir_all(&dep_dir).unwrap();

        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies.noconfig]\npath = \"noconfig\"\n",
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(
            result.is_ok(),
            "lsharp.toml がない依存先でもエラーにはならないべき"
        );

        // シンボリックリンクは作成されない
        let link_path = find_installed_package_dir(&base_dir, "noconfig");
        assert!(
            link_path.is_none(),
            "lsharp.toml がない依存先にはリンクを作らないべき"
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_install_version_dependency_uses_highest_compatible_cached_package() {
        let base_dir = std::env::temp_dir().join("lsharp_test_install_version_cached");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-a/src")).unwrap();
        std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-b/src")).unwrap();
        std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-c/src")).unwrap();

        std::fs::write(
            base_dir.join(".lsharp/packages/math-core-a/lsharp.toml"),
            "[project]\nname = \"math-core\"\nversion = \"1.0.1\"\n",
        )
        .unwrap();
        std::fs::write(
            base_dir.join(".lsharp/packages/math-core-b/lsharp.toml"),
            "[project]\nname = \"math-core\"\nversion = \"1.4.0\"\n",
        )
        .unwrap();
        std::fs::write(
            base_dir.join(".lsharp/packages/math-core-c/lsharp.toml"),
            "[project]\nname = \"math-core\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies]\nmath-core = \"1.0.0\"\n",
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(
            result.is_ok(),
            "cache からの semver 解決は成功するべき: {result:?}"
        );

        let lock = crate::lockfile::read_lockfile(&base_dir.join(".lsharp/lock.toml")).unwrap();
        let entry = lock
            .entries
            .iter()
            .find(|entry| entry.name == "math-core")
            .unwrap();
        assert_eq!(entry.version, "1.4.0");

        std::fs::remove_dir_all(&base_dir).unwrap();
    }

    #[test]
    fn test_cmd_install_version_dependency_errors_when_no_cached_match_exists() {
        let base_dir = std::env::temp_dir().join("lsharp_test_install_version_missing");
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(base_dir.join(".lsharp/packages/math-core-a/src")).unwrap();

        std::fs::write(
            base_dir.join(".lsharp/packages/math-core-a/lsharp.toml"),
            "[project]\nname = \"math-core\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();
        std::fs::write(
            base_dir.join("lsharp.toml"),
            "[dependencies]\nmath-core = \"1.0.0\"\n",
        )
        .unwrap();

        let result = cmd_install_in(&base_dir);
        assert!(
            result.is_err(),
            "一致する cache がない version 依存は失敗するべき"
        );

        std::fs::remove_dir_all(&base_dir).unwrap();
    }
}
