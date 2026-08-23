use crate::infer::Infer;
use crate::types::Type;
use lsharp_syntax::ast::{Decl, Expr, Pattern, Program};
use lsharp_syntax::span::Span;

use super::references::{
    collect_scoped_var_references, find_quote_span, is_builtin, span_contains,
};
use super::{MetadataDiagnostic, Severity};

struct LegacyInvariantProbe {
    name: String,
    owner: String,
    span: Span,
}

/// legacy `:invariant` を実際の関数戻り値の scope で Bool として検査する。
///
/// `result` は元関数を同じ引数で呼び出した値に束縛する synthetic probe を使う。
/// これにより、元関数の推論済み戻り値型を保ったまま metadata 式だけを検査できる。
pub(super) fn check_legacy_invariant_types(
    program: &Program,
    all_names: &[String],
) -> Vec<MetadataDiagnostic> {
    let mut check_program = program.clone();
    let mut probes = Vec::new();
    let mut quote_diagnostics = Vec::new();

    for decl in &program.decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        let Decl::Defn {
            name,
            params,
            metadata: Some(metadata),
            ..
        } = actual_decl
        else {
            continue;
        };
        let Some(invariant) = metadata.invariant.as_ref() else {
            continue;
        };

        // `I-59`: `:invariant` は生成ソースへ差し込まれて実行されるので、マクロ展開後に
        // 残らない quote が書かれていたら probe を組み立てずにここで弾く。
        // 型推論へ渡すと「未定義の変数」という原因と噛み合わない見出しになる。
        if let Some(quote_span) = find_quote_span(invariant) {
            quote_diagnostics.push(MetadataDiagnostic {
                severity: Severity::Error,
                message: ":invariant に quote/unquote は書けません (実行可能な contract であり、quote はマクロ展開後に残らないため)".to_string(),
                span: quote_span,
                function_name: name.clone(),
            });
            continue;
        }

        let param_names: Vec<&str> = params.iter().map(|param| param.name.as_str()).collect();
        let has_unknown_reference =
            collect_scoped_var_references(invariant)
                .iter()
                .any(|(ref_name, _)| {
                    !is_builtin(ref_name)
                        && ref_name != "result"
                        && !param_names.contains(&ref_name.as_str())
                        && !all_names.contains(ref_name)
                });
        if has_unknown_reference {
            continue;
        }

        let span = invariant.span();
        let call_args = params
            .iter()
            .map(|param| Expr::Var(param.span, param.name.clone()))
            .collect();
        let result_call = Expr::App(span, Box::new(Expr::Var(span, name.clone())), call_args);
        let probe_body = Expr::Let(
            span,
            vec![(Pattern::Var(span, "result".to_string()), result_call)],
            Box::new(invariant.clone()),
        );
        let probe_name = format!("__lsharp_legacy_invariant_{}", probes.len());
        check_program.decls.push(Decl::Defn {
            span,
            name: probe_name.clone(),
            params: params.clone(),
            return_ty: None,
            body: probe_body,
            where_clauses: Vec::new(),
            metadata: None,
        });
        probes.push(LegacyInvariantProbe {
            name: probe_name,
            owner: name.clone(),
            span,
        });
    }

    if probes.is_empty() {
        return quote_diagnostics;
    }

    let mut infer = Infer::new();
    let inferred = match infer.infer_program(&check_program) {
        Ok(results) => {
            let bool_type = Type::bool();
            probes
                .iter()
                .filter_map(|probe| {
                    let (_, scheme) = results.iter().find(|(name, _)| name == &probe.name)?;
                    let Type::Fun(_, return_type) = &scheme.ty else {
                        return None;
                    };
                    (return_type.as_ref() != &bool_type).then(|| MetadataDiagnostic {
                        severity: Severity::Error,
                        message: format!(
                            ":invariant は Bool 必須ですが、{} が推論されました",
                            return_type
                        ),
                        span: probe.span,
                        function_name: probe.owner.clone(),
                    })
                })
                .collect()
        }
        Err(error) => match error.span().and_then(|error_span| {
            probes
                .iter()
                .find(|probe| span_contains(probe.span, error_span))
                .map(|probe| (error_span, probe))
        }) {
            None => Vec::new(),
            Some((error_span, probe)) => vec![MetadataDiagnostic {
                severity: Severity::Error,
                message: format!(":invariant の型推論に失敗しました: {error}"),
                span: error_span,
                function_name: probe.owner.clone(),
            }],
        },
    };

    quote_diagnostics.extend(inferred);
    quote_diagnostics
}
