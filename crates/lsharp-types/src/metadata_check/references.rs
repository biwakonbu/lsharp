//! メタデータ式から参照とスコープ情報を収集する補助関数。

use lsharp_syntax::ast::{ComputationStep, Expr, Pattern};
use lsharp_syntax::span::Span;

pub(super) fn span_contains(outer: Span, inner: Span) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

/// :doc 文字列からバッククォート内の識別子を抽出
pub(super) fn extract_doc_identifiers(doc: &str) -> Vec<String> {
    let mut identifiers = Vec::new();
    let mut chars = doc.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '`' {
            let mut ident = String::new();
            for c in chars.by_ref() {
                if c == '`' {
                    break;
                }
                ident.push(c);
            }
            if !ident.is_empty() {
                identifiers.push(ident);
            }
        }
    }

    identifiers
}

/// 式から参照されている変数名を再帰的に収集
pub(super) fn collect_var_references(expr: &Expr) -> Vec<(String, Span)> {
    let mut refs = Vec::new();
    collect_var_references_inner(expr, &mut refs);
    refs
}

/// 式から変数参照を再帰的に収集（内部実装）
fn collect_var_references_inner(expr: &Expr, refs: &mut Vec<(String, Span)>) {
    match expr {
        Expr::Var(span, name) => {
            refs.push((name.clone(), *span));
        }
        Expr::Lit(_, _) => {}
        Expr::If(_, cond, then_branch, else_branch) => {
            collect_var_references_inner(cond, refs);
            collect_var_references_inner(then_branch, refs);
            collect_var_references_inner(else_branch, refs);
        }
        Expr::Let(_, bindings, body) => {
            for (_, expr) in bindings {
                collect_var_references_inner(expr, refs);
            }
            collect_var_references_inner(body, refs);
        }
        Expr::Lambda(_, _, body) => {
            collect_var_references_inner(body, refs);
        }
        Expr::App(_, func, args) => {
            collect_var_references_inner(func, refs);
            for arg in args {
                collect_var_references_inner(arg, refs);
            }
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_var_references_inner(scrutinee, refs);
            for arm in arms {
                collect_var_references_inner(&arm.body, refs);
            }
        }
        Expr::Do(_, exprs) => {
            for e in exprs {
                collect_var_references_inner(e, refs);
            }
        }
        Expr::Ann(_, inner, _) => {
            collect_var_references_inner(inner, refs);
        }
        Expr::RecordLit(_, _, fields) => {
            for (_, e) in fields {
                collect_var_references_inner(e, refs);
            }
        }
        Expr::FieldAccess(_, inner, _) => {
            collect_var_references_inner(inner, refs);
        }
        Expr::RecordUpdate(_, base, fields) => {
            collect_var_references_inner(base, refs);
            for (_, e) in fields {
                collect_var_references_inner(e, refs);
            }
        }
        Expr::Computation(_, _, steps) => {
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, _, expr) => {
                        collect_var_references_inner(expr, refs)
                    }
                    ComputationStep::DoBang(_, expr) => collect_var_references_inner(expr, refs),
                    ComputationStep::Return(_, expr) => collect_var_references_inner(expr, refs),
                    ComputationStep::Expr(expr) => collect_var_references_inner(expr, refs),
                }
            }
        }
        // P10-1 / `I-43`: quote の内側はデータであって参照ではない。
        // `~` / `~@` で戻した部分だけが本物の参照なので、そこだけ拾う。
        Expr::Quote(_, inner) => {
            collect_unquoted_references(inner, &mut |expr| {
                collect_var_references_inner(expr, refs)
            });
        }
        Expr::Unquote(_, inner) | Expr::UnquoteSplice(_, inner) => {
            collect_var_references_inner(inner, refs);
        }
    }
}

/// quote された式の中から `~` / `~@` で戻された部分式だけを取り出して `visit` へ渡す。
///
/// quote は入れ子になりうるが、`Expr::Quote` を跨いだ内側の unquote は
/// 外側の quote には戻らない。ここでは 1 段だけを扱い、入れ子の quote は素通しする。
fn collect_unquoted_references(expr: &Expr, visit: &mut impl FnMut(&Expr)) {
    match expr {
        Expr::Unquote(_, inner) | Expr::UnquoteSplice(_, inner) => visit(inner),
        Expr::Quote(_, _) => {}
        Expr::App(_, func, args) => {
            collect_unquoted_references(func, visit);
            for arg in args {
                collect_unquoted_references(arg, visit);
            }
        }
        Expr::If(_, cond, then_branch, else_branch) => {
            collect_unquoted_references(cond, visit);
            collect_unquoted_references(then_branch, visit);
            collect_unquoted_references(else_branch, visit);
        }
        Expr::Do(_, exprs) => {
            for inner in exprs {
                collect_unquoted_references(inner, visit);
            }
        }
        Expr::Ann(_, inner, _) | Expr::FieldAccess(_, inner, _) => {
            collect_unquoted_references(inner, visit);
        }
        Expr::Let(_, bindings, body) => {
            for (_, value) in bindings {
                collect_unquoted_references(value, visit);
            }
            collect_unquoted_references(body, visit);
        }
        Expr::Lambda(_, _, body) => collect_unquoted_references(body, visit),
        Expr::Match(_, scrutinee, arms) => {
            collect_unquoted_references(scrutinee, visit);
            for arm in arms {
                collect_unquoted_references(&arm.body, visit);
            }
        }
        Expr::RecordLit(_, _, fields) => {
            for (_, value) in fields {
                collect_unquoted_references(value, visit);
            }
        }
        Expr::RecordUpdate(_, base, fields) => {
            collect_unquoted_references(base, visit);
            for (_, value) in fields {
                collect_unquoted_references(value, visit);
            }
        }
        Expr::Computation(_, _, steps) => {
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, _, inner)
                    | ComputationStep::DoBang(_, inner)
                    | ComputationStep::Return(_, inner)
                    | ComputationStep::Expr(inner) => collect_unquoted_references(inner, visit),
                }
            }
        }
        Expr::Var(_, _) | Expr::Lit(_, _) => {}
    }
}

/// `I-59`: 式の中に最初に現れる quote / unquote / unquote-splice の span を返す。
///
/// `:invariant` は検査されるだけでなく `lsharp test` で**実行される** contract なので、
/// マクロ展開後に残らない quote が書かれていたら型推論へ渡す前に弾く。
/// 判断は `docs/adr/decisions-invariant-quote-handling.md`。
pub(super) fn find_quote_span(expr: &Expr) -> Option<Span> {
    match expr {
        Expr::Quote(span, _) | Expr::Unquote(span, _) | Expr::UnquoteSplice(span, _) => Some(*span),
        Expr::Var(_, _) | Expr::Lit(_, _) => None,
        Expr::App(_, func, args) => {
            find_quote_span(func).or_else(|| args.iter().find_map(find_quote_span))
        }
        Expr::If(_, cond, then_branch, else_branch) => find_quote_span(cond)
            .or_else(|| find_quote_span(then_branch))
            .or_else(|| find_quote_span(else_branch)),
        Expr::Do(_, exprs) => exprs.iter().find_map(find_quote_span),
        Expr::Ann(_, inner, _) | Expr::FieldAccess(_, inner, _) => find_quote_span(inner),
        Expr::Let(_, bindings, body) => bindings
            .iter()
            .find_map(|(_, value)| find_quote_span(value))
            .or_else(|| find_quote_span(body)),
        Expr::Lambda(_, _, body) => find_quote_span(body),
        Expr::Match(_, scrutinee, arms) => find_quote_span(scrutinee)
            .or_else(|| arms.iter().find_map(|arm| find_quote_span(&arm.body))),
        Expr::RecordLit(_, _, fields) => {
            fields.iter().find_map(|(_, value)| find_quote_span(value))
        }
        Expr::RecordUpdate(_, base, fields) => find_quote_span(base)
            .or_else(|| fields.iter().find_map(|(_, value)| find_quote_span(value))),
        Expr::Computation(_, _, steps) => steps.iter().find_map(|step| match step {
            ComputationStep::LetBang(_, _, inner)
            | ComputationStep::DoBang(_, inner)
            | ComputationStep::Return(_, inner)
            | ComputationStep::Expr(inner) => find_quote_span(inner),
        }),
    }
}

/// lexical scope を考慮して、式の自由変数参照だけを収集する。
pub(super) fn collect_scoped_var_references(expr: &Expr) -> Vec<(String, Span)> {
    let mut refs = Vec::new();
    let mut scope = Vec::new();
    collect_scoped_var_references_inner(expr, &mut scope, &mut refs);
    refs
}

fn collect_scoped_var_references_inner(
    expr: &Expr,
    scope: &mut Vec<String>,
    refs: &mut Vec<(String, Span)>,
) {
    match expr {
        Expr::Var(span, name) => {
            if !scope.iter().any(|bound| bound == name) {
                refs.push((name.clone(), *span));
            }
        }
        Expr::Lit(_, _) => {}
        Expr::If(_, cond, then_branch, else_branch) => {
            collect_scoped_var_references_inner(cond, scope, refs);
            collect_scoped_var_references_inner(then_branch, scope, refs);
            collect_scoped_var_references_inner(else_branch, scope, refs);
        }
        Expr::Let(_, bindings, body) => {
            let scope_start = scope.len();
            for (pattern, value) in bindings {
                collect_scoped_var_references_inner(value, scope, refs);
                collect_pattern_bindings(pattern, scope);
            }
            collect_scoped_var_references_inner(body, scope, refs);
            scope.truncate(scope_start);
        }
        Expr::Lambda(_, params, body) => {
            let scope_start = scope.len();
            scope.extend(params.iter().map(|param| param.name.clone()));
            collect_scoped_var_references_inner(body, scope, refs);
            scope.truncate(scope_start);
        }
        Expr::App(_, func, args) => {
            collect_scoped_var_references_inner(func, scope, refs);
            for arg in args {
                collect_scoped_var_references_inner(arg, scope, refs);
            }
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_scoped_var_references_inner(scrutinee, scope, refs);
            for arm in arms {
                let scope_start = scope.len();
                collect_pattern_bindings(&arm.pattern, scope);
                if let Some(guard) = &arm.guard {
                    collect_scoped_var_references_inner(guard, scope, refs);
                }
                collect_scoped_var_references_inner(&arm.body, scope, refs);
                scope.truncate(scope_start);
            }
        }
        Expr::Do(_, exprs) => {
            for expr in exprs {
                collect_scoped_var_references_inner(expr, scope, refs);
            }
        }
        Expr::Ann(_, inner, _) => collect_scoped_var_references_inner(inner, scope, refs),
        Expr::RecordLit(_, _, fields) => {
            for (_, value) in fields {
                collect_scoped_var_references_inner(value, scope, refs);
            }
        }
        Expr::FieldAccess(_, inner, _) => {
            collect_scoped_var_references_inner(inner, scope, refs);
        }
        Expr::RecordUpdate(_, base, fields) => {
            collect_scoped_var_references_inner(base, scope, refs);
            for (_, value) in fields {
                collect_scoped_var_references_inner(value, scope, refs);
            }
        }
        Expr::Computation(_, _, steps) => {
            let scope_start = scope.len();
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, pattern, value) => {
                        collect_scoped_var_references_inner(value, scope, refs);
                        collect_pattern_bindings(pattern, scope);
                    }
                    ComputationStep::DoBang(_, expr)
                    | ComputationStep::Return(_, expr)
                    | ComputationStep::Expr(expr) => {
                        collect_scoped_var_references_inner(expr, scope, refs)
                    }
                }
            }
            scope.truncate(scope_start);
        }
        // `I-43`: quote の内側はデータ。`~` / `~@` で戻した部分だけが参照。
        Expr::Quote(_, inner) => {
            let mut quoted = Vec::new();
            collect_unquoted_references(inner, &mut |expr| quoted.push(expr.clone()));
            for expr in &quoted {
                collect_scoped_var_references_inner(expr, scope, refs);
            }
        }
        Expr::Unquote(_, inner) | Expr::UnquoteSplice(_, inner) => {
            collect_scoped_var_references_inner(inner, scope, refs);
        }
    }
}

fn collect_pattern_bindings(pattern: &Pattern, scope: &mut Vec<String>) {
    match pattern {
        Pattern::Wildcard(_) | Pattern::Lit(_, _) => {}
        Pattern::Var(_, name) => scope.push(name.clone()),
        Pattern::Constructor(_, _, fields) => {
            for field in fields {
                collect_pattern_bindings(field, scope);
            }
        }
        Pattern::RecordPat(_, _, fields) => {
            for (_, field) in fields {
                collect_pattern_bindings(field, scope);
            }
        }
    }
}

/// 組み込み関数・演算子名（検証で除外する）
pub(super) fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "and"
            | "or"
            | "not"
            | "print"
            | "println"
            | "true"
            | "false"
            | "nil"
    )
}
