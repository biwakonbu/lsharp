mod config;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "lsharp", version, about = "L# コンパイラ")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// プロジェクトを初期化
    Init {
        /// プロジェクト名
        name: String,
    },

    /// ソースファイルをパースして AST を表示
    Parse {
        /// 入力ファイル
        file: PathBuf,

        /// AST を表示する
        #[arg(long)]
        ast: bool,
    },

    /// ソースファイルを型チェック
    Check {
        /// 入力ファイル
        file: PathBuf,

        /// Knowledge JSON を出力する
        #[arg(long)]
        emit_knowledge: bool,
    },

    /// ソースファイルを Wasm にコンパイル
    Compile {
        /// 入力ファイル
        file: PathBuf,

        /// 出力ファイル
        #[arg(short, long)]
        output: Option<PathBuf>,

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

    /// 対話的 REPL (Read-Eval-Print Loop)
    Repl,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Init { name } => {
            cmd_init(&name)?;
        }

        Command::Parse { file, ast } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

            let program = lsharp_syntax::parse(&source)
                .map_err(|e| miette::miette!("{e}"))?;

            if ast {
                println!("{program:#?}");
            } else {
                println!("{program}");
            }
        }

        Command::Check { file, emit_knowledge } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

            let program = lsharp_syntax::parse(&source)
                .map_err(|e| miette::miette!("{e}"))?;

            let mut infer = lsharp_types::infer::Infer::new();
            let results = infer
                .infer_program(&program)
                .map_err(|e| miette::miette!("{e}"))?;

            if emit_knowledge {
                let knowledge = build_knowledge(&program, &results, &infer);
                let json = knowledge.to_json()
                    .map_err(|e| miette::miette!("Knowledge JSON 生成失敗: {e}"))?;
                println!("{json}");
                return Ok(());
            }

            for (name, scheme) in &results {
                println!("{name} : {scheme}");
            }

            // メタデータ検証
            let diagnostics = lsharp_types::metadata_check::check_metadata(&program);
            if !diagnostics.is_empty() {
                println!("\nメタデータ検証:");
                for diag in &diagnostics {
                    println!("  {diag}");
                }
            }

            println!("\n型チェック成功 ({} 個の定義)", results.len());
        }

        Command::Compile {
            file,
            output,
            emit_ir,
        }
        | Command::Build {
            file,
            output,
            emit_ir,
        } => {
            // P0-1: git リポジトリ必須チェック
            check_git_repo(&file)?;

            // lsharp.toml 設定読み込み
            let project_dir = file.parent().unwrap_or(std::path::Path::new("."));
            let _config = config::load_config(project_dir);

            // P6: マルチファイルコンパイル対応
            // ファイルに (import ...) が含まれているか確認し、
            // あればマルチファイルモードで依存関係を解決する
            let module = if has_file_imports(&file)? {
                // マルチファイルコンパイル
                lsharp_ir::compile_multi_file(&file)
                    .map_err(|e| miette::miette!("{e}"))?
            } else {
                // 単一ファイルコンパイル（従来通り）
                let source = std::fs::read_to_string(&file)
                    .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
                let program = lsharp_syntax::parse(&source)
                    .map_err(|e| miette::miette!("{e}"))?;
                let mut infer = lsharp_types::infer::Infer::new();
                let type_results = infer
                    .infer_program(&program)
                    .map_err(|e| miette::miette!("{e}"))?;
                let mut lower = lsharp_ir::lower::Lower::new();
                lower
                    .lower_program(&program, &type_results)
                    .map_err(|e| miette::miette!("{e}"))?
            };

            if emit_ir {
                print!("{}", module.dump());
                return Ok(());
            }

            // Wasm コード生成 (WASI モード: wasmtime で直接実行可能)
            let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
                .map_err(|e| miette::miette!("{e}"))?;

            // 出力
            let output_path = output.unwrap_or_else(|| {
                file.with_extension("wasm")
            });

            std::fs::write(&output_path, &wasm_bytes)
                .map_err(|e| miette::miette!("{}: {}", output_path.display(), e))?;

            println!(
                "コンパイル成功: {} ({} bytes)",
                output_path.display(),
                wasm_bytes.len()
            );
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
            let program = lsharp_syntax::parse(&source)
                .map_err(|e| miette::miette!("{e}"))?;
            let metadata_diagnostics = lsharp_types::metadata_check::check_metadata(&program);
            let diag_strings: Vec<String> = metadata_diagnostics
                .iter()
                .map(|d| d.to_string())
                .collect();

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

        Command::DocCheck { file, skip_doc_review, emit_trailers } => {
            if skip_doc_review {
                println!("ドキュメントレビューをスキップしました");
                return Ok(());
            }

            let source = std::fs::read_to_string(&file)
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

            // パースとメタデータ検証
            let program = lsharp_syntax::parse(&source)
                .map_err(|e| miette::miette!("{e}"))?;
            let diagnostics = lsharp_types::metadata_check::check_metadata(&program);

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
            for diag in &diagnostics {
                let diag_str = diag.to_string();
                if diag_str.contains("[Error]") {
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
                let trailer_status = if has_errors {
                    "Failed"
                } else {
                    "Passed"
                };
                println!("Doc-Review-Status: {trailer_status}");

                // レビュー済みエントリのレビュー者を収集
                let mut reviewers: Vec<String> = doc_status.entries.values()
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

        Command::Repl => {
            cmd_repl()?;
        }
    }

    Ok(())
}

/// P3-3: メタデータテスト実行 (:example, :invariant の自動検証)
fn cmd_test(file: &PathBuf) -> miette::Result<()> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

    // パース
    let program = lsharp_syntax::parse(&source)
        .map_err(|e| miette::miette!("{e}"))?;

    // テストケース生成
    let tests = lsharp_types::metadata_check::generate_tests(&program);

    if tests.is_empty() {
        println!("テストなし: {} にはテスト対象のメタデータがありません", file.display());
        return Ok(());
    }

    println!(
        "テスト実行: {} ({} テスト)",
        file.display(),
        tests.len()
    );

    // テスト用プログラムを生成
    let test_source = lsharp_wasm::test_runner::generate_test_program(&program, &tests);

    // コンパイル
    let test_program = lsharp_syntax::parse(&test_source)
        .map_err(|e| miette::miette!("テストプログラムのパースに失敗: {e}"))?;

    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&test_program)
        .map_err(|e| miette::miette!("テストプログラムの型チェックに失敗: {e}"))?;

    let mut lower = lsharp_ir::lower::Lower::new();
    let module = lower
        .lower_program(&test_program, &type_results)
        .map_err(|e| miette::miette!("テストプログラムの IR 変換に失敗: {e}"))?;

    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| miette::miette!("テストプログラムの Wasm 生成に失敗: {e}"))?;

    // WASI 環境で実行
    let output = run_wasm_wasi(&wasm_bytes)
        .map_err(|e| miette::miette!("テスト実行に失敗: {e}"))?;

    // 結果を解析
    let results = lsharp_wasm::test_runner::parse_test_output(&output, &tests, &program);

    // 結果表示
    let mut passed = 0;
    let mut failed = 0;

    for result in &results {
        let kind_str = match result.kind {
            lsharp_types::metadata_check::TestKind::Example => "example",
            lsharp_types::metadata_check::TestKind::Invariant => "invariant",
        };

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
        passed + failed,
        passed,
        failed
    );

    if failed > 0 {
        return Err(miette::miette!(
            "{failed} 個のテストが失敗しました"
        ));
    }

    Ok(())
}

/// Wasm バイナリを WASI 環境で実行し、stdout 出力を返す
fn run_wasm_wasi(wasm_bytes: &[u8]) -> Result<String, String> {
    lsharp_wasm::wasi_runner::run_wasm_wasi(wasm_bytes)
}

/// Knowledge JSON を構築
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
    let is_private_set: std::collections::HashSet<&str> =
        infer.module_env.privates.iter().map(|s| s.as_str()).collect();

    // Private 名の収集のために再利用
    let _ = &is_private_set;

    for decl in &program.decls {
        // Private を展開
        let (actual_decl, is_priv) = match decl {
            Decl::Private { inner, .. } => (inner.as_ref(), true),
            other => (other, false),
        };

        match actual_decl {
            Decl::Defn { name, params, metadata, .. } => {
                let type_str = type_results.iter()
                    .find(|(n, _)| n == name)
                    .map(|(_, s)| format!("{}", s.ty))
                    .unwrap_or_else(|| "?".to_string());

                let param_infos: Vec<ParamInfo> = params.iter().map(|p| {
                    let desc = metadata.as_ref().and_then(|m| {
                        m.params.iter().find(|(n, _)| n == &p.name).map(|(_, d)| d.clone())
                    });
                    ParamInfo {
                        name: p.name.clone(),
                        ty: p.ty.as_ref().map(|t| format!("{t}")).unwrap_or_else(|| "?".to_string()),
                        description: desc,
                    }
                }).collect();

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
                let field_infos: Vec<FieldInfo> = fields.iter().map(|(fname, ftype)| {
                    FieldInfo {
                        name: fname.clone(),
                        ty: format!("{ftype}"),
                    }
                }).collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Record { fields: field_infos },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TypeDef { name, variants, .. } => {
                let variant_infos: Vec<VariantInfo> = variants.iter().map(|v| {
                    VariantInfo {
                        name: v.name.clone(),
                        fields: v.fields.iter().map(|f| format!("{f}")).collect(),
                    }
                }).collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Adt { variants: variant_infos },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TypeAlias { name, target, .. } => {
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Alias { target: format!("{target}") },
                    type_params: vec![],
                    doc: None,
                });
            }
            Decl::TraitDef { name, methods, .. } => {
                let method_names: Vec<String> = methods.iter().map(|m| m.name.clone()).collect();
                types.push(TypeInfo {
                    name: name.clone(),
                    kind: TypeKind::Trait { methods: method_names },
                    type_params: vec![],
                    doc: None,
                });
            }
            _ => {}
        }
    }

    // 依存関係
    let dependencies: Vec<DependencyInfo> = infer.module_env.imports.iter().map(|imp| {
        let kind = if imp.open {
            DependencyKind::OpenImport
        } else if let Some(ref only) = imp.only {
            DependencyKind::SelectiveImport { symbols: only.clone() }
        } else {
            DependencyKind::Import
        };
        DependencyInfo {
            from: module_name.clone().unwrap_or_else(|| "main".to_string()),
            to: imp.module.clone(),
            kind,
        }
    }).collect();

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

/// P6: ファイルにモジュール import が含まれているか確認
///
/// `(import ModuleName)` 宣言があれば true を返す。
/// マルチファイルコンパイルの切り替え判定に使用。
fn has_file_imports(file: &std::path::Path) -> miette::Result<bool> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

    let program = lsharp_syntax::parse(&source)
        .map_err(|e| miette::miette!("{e}"))?;

    for decl in &program.decls {
        if matches!(decl, lsharp_syntax::ast::Decl::ImportDecl { .. }) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// P0-1: git リポジトリの存在を検証
fn check_git_repo(file: &std::path::Path) -> miette::Result<()> {
    // ファイルの親ディレクトリから .git を探索
    let mut dir = file.canonicalize()
        .unwrap_or_else(|_| file.to_path_buf());

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
    use std::fs;
    use std::process::Command as ProcessCommand;

    let project_dir = PathBuf::from(name);

    // ディレクトリ作成
    if project_dir.exists() {
        return Err(miette::miette!(
            "ディレクトリ '{name}' は既に存在します"
        ));
    }

    fs::create_dir_all(&project_dir)
        .map_err(|e| miette::miette!("ディレクトリ作成失敗: {e}"))?;

    // src ディレクトリ作成
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)
        .map_err(|e| miette::miette!("src ディレクトリ作成失敗: {e}"))?;

    // lsharp.toml 生成
    let toml_content = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"
"#
    );
    fs::write(project_dir.join("lsharp.toml"), toml_content)
        .map_err(|e| miette::miette!("lsharp.toml 作成失敗: {e}"))?;

    // main.ls 生成
    let main_content = r#"(defn main []
  (print 42))
"#;
    fs::write(src_dir.join("main.ls"), main_content)
        .map_err(|e| miette::miette!("main.ls 作成失敗: {e}"))?;

    // .gitignore 生成
    let gitignore_content = "*.wasm\n/target/\n";
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
    println!("  {}/src/main.ls", name);
    println!("\n次のステップ:");
    println!("  cd {name}");
    println!("  lsharp compile src/main.ls");

    Ok(())
}

/// P9-1: 対話的 REPL
fn cmd_repl() -> miette::Result<()> {
    use std::io::{self, BufRead, Write};

    println!("L# REPL v0.1.0");
    println!("式を入力してください。終了するには Ctrl+D を押してください。");
    println!();

    let stdin = io::stdin();
    let mut infer = lsharp_types::infer::Infer::new();
    let mut expr_count = 0;

    loop {
        print!("lsharp> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            println!();
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // 式を main 関数でラップしてコンパイル・実行
        let source = format!("(defn main [] {})", line);

        match lsharp_syntax::parse(&source) {
            Ok(program) => {
                // 新しい Infer インスタンスを毎回作成 (状態リセット)
                let mut local_infer = lsharp_types::infer::Infer::new();
                match local_infer.infer_program(&program) {
                    Ok(type_results) => {
                        let mut lower = lsharp_ir::lower::Lower::new();
                        match lower.lower_program(&program, &type_results) {
                            Ok(module) => {
                                match lsharp_wasm::wasi::emit_wasm_wasi(&module) {
                                    Ok(wasm_bytes) => {
                                        match lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes) {
                                            Ok(output) => {
                                                let output = output.trim();
                                                if !output.is_empty() {
                                                    println!("{}", output);
                                                }
                                                expr_count += 1;
                                            }
                                            Err(e) => eprintln!("実行エラー: {}", e),
                                        }
                                    }
                                    Err(e) => eprintln!("コード生成エラー: {}", e),
                                }
                            }
                            Err(e) => eprintln!("IR 変換エラー: {}", e),
                        }
                    }
                    Err(e) => eprintln!("型エラー: {}", e),
                }
            }
            Err(e) => eprintln!("パースエラー: {}", e),
        }
    }

    let _ = infer; // suppress unused warning
    println!("セッション終了。{} 式を評価しました。", expr_count);
    Ok(())
}
