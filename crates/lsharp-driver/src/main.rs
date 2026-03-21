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
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.command {
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

        Command::Check { file } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

            let program = lsharp_syntax::parse(&source)
                .map_err(|e| miette::miette!("{e}"))?;

            let mut infer = lsharp_types::infer::Infer::new();
            let results = infer
                .infer_program(&program)
                .map_err(|e| miette::miette!("{e}"))?;

            for (name, scheme) in &results {
                println!("{name} : {scheme}");
            }

            println!("\n型チェック成功 ({} 個の定義)", results.len());
        }

        Command::Compile {
            file,
            output,
            emit_ir,
        } => {
            let source = std::fs::read_to_string(&file)
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

            // パース
            let program = lsharp_syntax::parse(&source)
                .map_err(|e| miette::miette!("{e}"))?;

            // 型チェック
            let mut infer = lsharp_types::infer::Infer::new();
            let type_results = infer
                .infer_program(&program)
                .map_err(|e| miette::miette!("{e}"))?;

            // IR 変換
            let mut lower = lsharp_ir::lower::Lower::new();
            let module = lower
                .lower_program(&program, &type_results)
                .map_err(|e| miette::miette!("{e}"))?;

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
    }

    Ok(())
}
