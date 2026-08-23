use lsharp_syntax::ast::{Expr, Metadata, Param};
use lsharp_syntax::span::Span;

use super::references::{
    collect_scoped_var_references, collect_var_references, extract_doc_identifiers,
    find_quote_span, is_builtin,
};
use super::{MetadataDiagnostic, Severity};

/// 関数定義のメタデータを検証
pub(super) fn check_defn_metadata(
    diagnostics: &mut Vec<MetadataDiagnostic>,
    name: &str,
    params: &[Param],
    metadata: &Metadata,
    span: Span,
    all_names: &[String],
) {
    let param_names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();

    // :params の検証
    if !metadata.params.is_empty() {
        // P3-3-1: :params キーと引数リストの一致チェック（エラー）
        for (meta_param, _desc) in &metadata.params {
            if !param_names.contains(&meta_param.as_str()) {
                diagnostics.push(MetadataDiagnostic {
                    severity: Severity::Error,
                    message: format!(":params に存在しない引数 '{meta_param}' が記載されています"),
                    span,
                    function_name: name.to_string(),
                });
            }
        }

        // P3-3-2: :params の全引数網羅チェック（警告）
        let meta_param_names: Vec<&str> = metadata.params.iter().map(|(n, _)| n.as_str()).collect();
        for param_name in &param_names {
            if !meta_param_names.contains(param_name) {
                diagnostics.push(MetadataDiagnostic {
                    severity: Severity::Warning,
                    message: format!("引数 '{param_name}' が :params に記載されていません"),
                    span,
                    function_name: name.to_string(),
                });
            }
        }
    }

    // P3-3-3: :see-also 参照先の存在チェック（エラー）
    for ref_name in &metadata.see_also {
        if !all_names.contains(ref_name) {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":see-also に存在しない識別子 '{ref_name}' が参照されています"),
                span,
                function_name: name.to_string(),
            });
        }
    }

    // P3-3-4: :doc 内のバッククォート識別子存在チェック（警告）
    if let Some(ref doc) = metadata.doc {
        let doc_idents = extract_doc_identifiers(doc);
        for ident in &doc_idents {
            // `I-43`: builtin を warning にしない。`:invariant` / `:example` と同じ扱いに揃える。
            if is_builtin(ident) {
                continue;
            }
            // 引数名、関数名、型名のいずれにも存在しない場合は警告
            if !param_names.contains(&ident.as_str()) && !all_names.contains(ident) {
                diagnostics.push(MetadataDiagnostic {
                    severity: Severity::Warning,
                    message: format!(":doc 内の識別子 `{ident}` がプログラム中に見つかりません"),
                    span,
                    function_name: name.to_string(),
                });
            }
        }
    }

    // P3-3-5: :invariant の検証
    if let Some(ref invariant_expr) = metadata.invariant {
        check_invariant(diagnostics, name, &param_names, invariant_expr, all_names);
    }

    // P3-3-6: :example の検証
    for example_expr in &metadata.example {
        check_example(
            diagnostics,
            name,
            &param_names,
            example_expr,
            span,
            all_names,
        );
    }
}

/// :invariant 式の構造検証
///
/// - 不変条件式内で参照されている変数が、関数の引数または既知の関数名であることを確認
/// - 不変条件式が空でないことを確認
fn check_invariant(
    diagnostics: &mut Vec<MetadataDiagnostic>,
    fn_name: &str,
    param_names: &[&str],
    invariant: &Expr,
    all_names: &[String],
) {
    let var_refs = collect_scoped_var_references(invariant);

    for (ref_name, ref_span) in &var_refs {
        // 組み込み演算子・関数はスキップ
        if is_builtin(ref_name) {
            continue;
        }
        // 「result」は暗黙の戻り値参照として許可
        if ref_name == "result" {
            continue;
        }
        // 引数名または既知の関数/型名にない場合はエラー
        if !param_names.contains(&ref_name.as_str()) && !all_names.contains(ref_name) {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":invariant 内で未定義の識別子 '{ref_name}' が参照されています"),
                span: *ref_span,
                function_name: fn_name.to_string(),
            });
        }
    }
}

/// :example 式の構造検証
///
/// - 例示式に quote/unquote が含まれていないことを確認
/// - 例示式内で参照されている変数が、関数の引数または既知の関数名であることを確認
fn check_example(
    diagnostics: &mut Vec<MetadataDiagnostic>,
    fn_name: &str,
    param_names: &[&str],
    example: &Expr,
    span: Span,
    all_names: &[String],
) {
    // `I-62`: `:example` は `test_runner.rs:78` で生成ソースへ差し込まれて実行されるので、
    // マクロ展開後に残らない quote が書かれていたらここで弾く。lowering まで通すと
    // 「未定義の変数」という原因と噛み合わない見出しになり、span も生成ソースを指す。
    // 検出は式全体。lowering (`ir/lower/expr/quote_expr.rs:9`) が位置によらず拒否するため、
    // 部分的に許す検出範囲は選べない。判断は
    // `docs/adr/decisions-example-quote-handling.md`。
    if let Some(quote_span) = find_quote_span(example) {
        diagnostics.push(MetadataDiagnostic {
            severity: Severity::Error,
            message: ":example に quote/unquote は書けません (実行される例であり、quote はマクロ展開後に残らないため)".to_string(),
            span: quote_span,
            function_name: fn_name.to_string(),
        });
    }

    let var_refs = collect_var_references(example);

    for (ref_name, _ref_span) in &var_refs {
        // 組み込み演算子・関数はスキップ
        if is_builtin(ref_name) {
            continue;
        }
        // 引数名または既知の関数/型名にない場合はエラー
        if !param_names.contains(&ref_name.as_str()) && !all_names.contains(ref_name) {
            diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":example 内で未定義の識別子 '{ref_name}' が参照されています"),
                span,
                function_name: fn_name.to_string(),
            });
        }
    }
}
