/// ソース文字列に対して metadata diagnostics を文字列化して返す。
pub fn check_metadata_strings(source: &str) -> miette::Result<Vec<String>> {
    let program = lsharp_syntax::parse(source).map_err(|e| miette::miette!("{e}"))?;
    Ok(lsharp_types::metadata_check::check_metadata(&program)
        .into_iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect())
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn migration_diagnostic_strings_preserve_selected_semantics_and_disposition() {
        let diagnostics = migration_diagnostic_strings(
            r#"
(defn succ [x]
  :example [(succ 0) (= (succ 1) 2)]
  :invariant (= result (+ x 1))
  (+ x 1))
"#,
        )
        .expect("legacy metadata migration diagnostics should be rendered");

        assert_eq!(diagnostics.len(), 3);
        assert!(diagnostics[0].starts_with("[LS2001]"));
        assert!(diagnostics[0].contains("disposition=docs-only-example"));
        assert!(diagnostics[1].contains("disposition=assertion"));
        assert!(diagnostics[2].starts_with("[LS2002]"));
        assert!(diagnostics[2].contains("disposition=property-postcondition"));
    }
}

/// legacy metadata を source order の migration report へ文字列化する。
pub fn migration_diagnostic_strings(source: &str) -> miette::Result<Vec<String>> {
    let program = lsharp_syntax::parse(source).map_err(|e| miette::miette!("{e}"))?;
    let diagnostics = lsharp_types::metadata_migration::classify_legacy_contracts(&program)
        .map_err(|e| miette::miette!("{e}"))?;
    Ok(diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_metadata_strings_returns_empty_for_valid_metadata() {
        let diagnostics = check_metadata_strings(
            r#"(defn abs [x] :example [(= (abs 5) 5)] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"#,
        )
        .expect("valid metadata should parse");
        assert!(
            diagnostics.is_empty(),
            "valid metadata should not produce diagnostics"
        );
    }

    #[test]
    fn test_check_metadata_strings_reports_invalid_metadata_error() {
        let diagnostics = check_metadata_strings(
            r#"(defn abs [x] :invariant (unknown-fn result) (if (< x 0) (- 0 x) x))"#,
        )
        .expect("invalid metadata source should still parse");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].contains("unknown-fn"));
        assert!(diagnostics[0].contains(":invariant"));
        assert!(diagnostics[0].contains("[error]"));
    }
}
