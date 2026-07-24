use std::path::Path;

use crate::diagnostics::driver_io_error;

/// ソースコードのフォーマット結果
#[allow(dead_code)]
pub enum FmtResult {
    /// フォーマット済み（変更なし）
    Unchanged,
    /// フォーマットで変更あり
    Changed {
        /// フォーマット後のソースコード
        formatted: String,
    },
}

/// L# ソースコードをフォーマットする
///
/// パースで妥当性だけを確認し、source-aware formatter で文字列リテラルを保持したまま整形する。
/// 末尾改行を付与して返す。
pub fn format_source(source: &str) -> Result<String, String> {
    lsharp_syntax::parse(source).map_err(|e| format!("{e}"))?;
    Ok(lsharp_lsp::format_source(source))
}

/// ソースのフォーマット差分を検出する
#[allow(dead_code)]
pub fn check_format(source: &str) -> Result<FmtResult, String> {
    let formatted = format_source(source)?;
    if formatted == source {
        Ok(FmtResult::Unchanged)
    } else {
        Ok(FmtResult::Changed { formatted })
    }
}

/// fmt サブコマンドのエントリポイント
#[allow(dead_code)]
pub fn cmd_fmt(file: &Path, check: bool, write: bool) -> miette::Result<()> {
    let source = std::fs::read_to_string(file)
        .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;

    let formatted = format_source(&source).map_err(|e| miette::miette!("フォーマット失敗: {e}"))?;

    if check {
        // --check モード: 差分があれば非ゼロ終了
        if formatted != source {
            return Err(miette::miette!(
                "{}: フォーマットの差分があります",
                file.display()
            ));
        }
        println!("{}: フォーマット済み", file.display());
        return Ok(());
    }

    if write {
        // --write モード: ファイルを上書き
        if formatted != source {
            std::fs::write(file, &formatted)
                .map_err(|e| driver_io_error(format!("{}: {}", file.display(), e)))?;
            println!("{}: フォーマット完了", file.display());
        } else {
            println!("{}: 変更なし", file.display());
        }
        return Ok(());
    }

    // デフォルト: stdout に出力
    print!("{formatted}");
    Ok(())
}

#[cfg(test)]
#[path = "fmt_tests.rs"]
mod tests;
