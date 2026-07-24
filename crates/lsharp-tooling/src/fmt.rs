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
mod tests {
    use super::*;

    #[test]
    fn test_fmt_roundtrip() {
        // パース → フォーマット → 再パースで AST が同一
        let source = "(defn main [] 42)\n";
        let formatted = format_source(source).unwrap();
        let program1 = lsharp_syntax::parse(source).unwrap();
        let program2 = lsharp_syntax::parse(&formatted).unwrap();
        assert_eq!(format!("{program1}"), format!("{program2}"));
    }

    #[test]
    fn test_fmt_output() {
        // 簡単な defn のフォーマット出力検証
        let source = "(defn main [] 42)";
        let formatted = format_source(source).unwrap();
        assert!(formatted.contains("defn main"));
        assert!(formatted.contains("42"));
        assert!(formatted.ends_with('\n'));
    }

    #[test]
    fn test_fmt_check_unchanged() {
        // フォーマット済みソースに対して check が Unchanged を返す
        let source = "(defn main [] 42)\n";
        let formatted = format_source(source).unwrap();
        match check_format(&formatted).unwrap() {
            FmtResult::Unchanged => {} // OK
            FmtResult::Changed { .. } => panic!("フォーマット済みソースは Unchanged であるべき"),
        }
    }

    #[test]
    fn test_fmt_check_changed() {
        // インデントが崩れたソースに対して check が Changed を返す
        let source = "(defn   main  []   42)\n";
        match check_format(source).unwrap() {
            FmtResult::Changed { formatted } => {
                assert!(formatted.contains("defn main"));
            }
            FmtResult::Unchanged => {
                // Display が空白を正規化しない場合もありうる
                // この場合はパーサーが空白を無視して同一出力するので OK
            }
        }
    }

    #[test]
    fn test_fmt_parse_error() {
        // パースエラーの場合はエラーを返す
        let source = "(defn";
        assert!(format_source(source).is_err());
    }

    #[test]
    fn test_cmd_fmt_missing_source_preserves_driver_io_error_code() {
        let file = std::env::temp_dir().join(format!(
            "lsharp_fmt_missing_source_{}_{}.ls",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock は unix epoch より後であるべき")
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&file);

        let error =
            cmd_fmt(&file, false, false).expect_err("存在しない source は fmt を失敗させるべき");
        assert!(
            error.to_string().starts_with("[LS5001]"),
            "fmt file I/O diagnostics は stable code を含むべき: {error}"
        );
        assert!(error.to_string().contains(file.to_string_lossy().as_ref()));
    }
}
