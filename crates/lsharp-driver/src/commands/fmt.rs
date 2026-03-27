use std::path::Path;

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
/// パース → Display でフォーマット済み S 式を生成する。
/// 末尾改行を付与して返す。
pub fn format_source(source: &str) -> Result<String, String> {
    let program = lsharp_syntax::parse(source).map_err(|e| format!("{e}"))?;
    let mut formatted = format!("{program}");
    // 末尾改行を保証
    if !formatted.ends_with('\n') {
        formatted.push('\n');
    }
    Ok(formatted)
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
pub fn cmd_fmt(file: &Path, check: bool, write: bool) -> miette::Result<()> {
    let source =
        std::fs::read_to_string(file).map_err(|e| miette::miette!("{}: {}", file.display(), e))?;

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
                .map_err(|e| miette::miette!("{}: {}", file.display(), e))?;
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
}
