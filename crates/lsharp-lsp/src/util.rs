use tower_lsp::lsp_types::*;

use lsharp_syntax::ast::{ComputationStep, Decl, Expr, Pattern, Program};
use lsharp_syntax::span::Span;

/// バイトオフセットを LSP Position (行・列) に変換する
pub(crate) fn offset_to_position(source: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if i + ch.len_utf8() > offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf16() as u32;
        }
    }
    Position::new(line, col)
}

/// LSP Position (行・列) をバイトオフセットに変換する
pub(crate) fn position_to_offset(source: &str, position: Position) -> Option<usize> {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if line == position.line && col == position.character {
            return Some(i);
        }
        if ch == '\n' {
            if line == position.line {
                // 行末を超えた場合、その行の末尾を返す
                return Some(i);
            }
            line += 1;
            col = 0;
        } else {
            let next_col = col + ch.len_utf16() as u32;
            if line == position.line && position.character < next_col {
                // サロゲートペアの途中など、文字境界でない位置は文字の先頭へ寄せる
                return Some(i);
            }
            col = next_col;
        }
    }
    // ファイル末尾
    if line == position.line && col == position.character {
        Some(source.len())
    } else {
        None
    }
}

/// シンボル定義情報
#[derive(Debug, Clone)]
pub(crate) struct SymbolDef {
    /// シンボル名
    pub name: String,
    /// 定義位置のバイトオフセット (開始)
    pub start: usize,
    /// 定義位置のバイトオフセット (終了)
    pub end: usize,
}

/// シンボル使用情報
#[derive(Debug, Clone)]
pub(crate) struct SymbolUsage {
    /// シンボル名
    pub name: String,
    /// 使用位置のバイトオフセット (開始)
    pub start: usize,
    /// 使用位置のバイトオフセット (終了)
    pub end: usize,
}

fn is_symbol_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '?' || c == '!'
}

/// 指定位置のシンボル名を取得する
pub(crate) fn symbol_at_position(source: &str, offset: usize) -> Option<String> {
    if offset >= source.len() {
        return None;
    }

    let ch = source[offset..].chars().next()?;
    if !is_symbol_char(ch) {
        return None;
    }

    // シンボルの開始位置を逆方向に探す
    let mut start = offset;
    while start > 0 {
        let prev_ch = source[..start].chars().next_back()?;
        if !is_symbol_char(prev_ch) {
            break;
        }
        start -= prev_ch.len_utf8();
    }

    // シンボルの終了位置を前方向に探す
    let mut end = offset;
    while end < source.len() {
        let next_ch = source[end..].chars().next()?;
        if !is_symbol_char(next_ch) {
            break;
        }
        end += next_ch.len_utf8();
    }

    if start == end {
        return None;
    }

    Some(source[start..end].to_string())
}

/// 指定位置のシンボル名と範囲 (バイトオフセット) を取得する
pub(crate) fn symbol_range_at_position(
    source: &str,
    offset: usize,
) -> Option<(String, usize, usize)> {
    if offset >= source.len() {
        return None;
    }

    let ch = source[offset..].chars().next()?;
    if !is_symbol_char(ch) {
        return None;
    }

    let mut start = offset;
    while start > 0 {
        let prev_ch = source[..start].chars().next_back()?;
        if !is_symbol_char(prev_ch) {
            break;
        }
        start -= prev_ch.len_utf8();
    }

    let mut end = offset;
    while end < source.len() {
        let next_ch = source[end..].chars().next()?;
        if !is_symbol_char(next_ch) {
            break;
        }
        end += next_ch.len_utf8();
    }

    if start == end {
        return None;
    }

    Some((source[start..end].to_string(), start, end))
}

/// AST からシンボル定義を収集する
pub(crate) fn collect_definitions(program: &Program, source: &str) -> Vec<SymbolDef> {
    let mut defs = Vec::new();
    for decl in &program.decls {
        collect_decl_definitions(decl, source, &mut defs);
    }
    defs
}

fn symbol_span_in_source(source: &str, span: Span, name: &str) -> Span {
    let start = span.start.min(source.len());
    let end = span.end.min(source.len());
    if start >= end || !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Span::new(start, end);
    }

    let mut search_start = start;
    while search_start < end {
        let Some(relative_start) = source[search_start..end].find(name) else {
            break;
        };
        let candidate_start = search_start + relative_start;
        let candidate_end = candidate_start + name.len();
        let preceded_by_symbol = source[..candidate_start]
            .chars()
            .next_back()
            .is_some_and(is_symbol_char);
        let followed_by_symbol = source[candidate_end..]
            .chars()
            .next()
            .is_some_and(is_symbol_char);
        if !preceded_by_symbol && !followed_by_symbol {
            return Span::new(candidate_start, candidate_end);
        }
        search_start = candidate_end;
    }

    Span::new(start, end)
}

/// 宣言からシンボル定義を収集する
fn collect_decl_definitions(decl: &Decl, source: &str, defs: &mut Vec<SymbolDef>) {
    match decl {
        Decl::Defn {
            span,
            name,
            params,
            body,
            ..
        } => {
            // 関数名の定義位置
            let name_span = symbol_span_in_source(source, *span, name);
            defs.push(SymbolDef {
                name: name.clone(),
                start: name_span.start,
                end: name_span.end,
            });
            // パラメータの定義位置
            for param in params {
                let param_span = symbol_span_in_source(source, param.span, &param.name);
                defs.push(SymbolDef {
                    name: param.name.clone(),
                    start: param_span.start,
                    end: param_span.end,
                });
            }
            // body 内の let バインディングを収集
            collect_expr_definitions(body, source, defs);
        }
        Decl::Private { inner, .. } => {
            collect_decl_definitions(inner, source, defs);
        }
        _ => {}
    }
}

/// 式内の let バインディング等からシンボル定義を収集する
fn collect_expr_definitions(expr: &Expr, source: &str, defs: &mut Vec<SymbolDef>) {
    match expr {
        Expr::Let(_, bindings, body) => {
            for (pat, val) in bindings {
                collect_pattern_definitions(pat, source, defs);
                collect_expr_definitions(val, source, defs);
            }
            collect_expr_definitions(body, source, defs);
        }
        Expr::If(_, cond, then_br, else_br) => {
            collect_expr_definitions(cond, source, defs);
            collect_expr_definitions(then_br, source, defs);
            collect_expr_definitions(else_br, source, defs);
        }
        Expr::App(_, func, args) => {
            collect_expr_definitions(func, source, defs);
            for arg in args {
                collect_expr_definitions(arg, source, defs);
            }
        }
        Expr::Ann(_, expr, _) => {
            collect_expr_definitions(expr, source, defs);
        }
        Expr::RecordLit(_, _, fields) => {
            for (_, value) in fields {
                collect_expr_definitions(value, source, defs);
            }
        }
        Expr::FieldAccess(_, expr, _) => {
            collect_expr_definitions(expr, source, defs);
        }
        Expr::RecordUpdate(_, expr, fields) => {
            collect_expr_definitions(expr, source, defs);
            for (_, value) in fields {
                collect_expr_definitions(value, source, defs);
            }
        }
        Expr::Lambda(_, params, body) => {
            for param in params {
                let param_span = symbol_span_in_source(source, param.span, &param.name);
                defs.push(SymbolDef {
                    name: param.name.clone(),
                    start: param_span.start,
                    end: param_span.end,
                });
            }
            collect_expr_definitions(body, source, defs);
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_expr_definitions(scrutinee, source, defs);
            for arm in arms {
                collect_pattern_definitions(&arm.pattern, source, defs);
                collect_expr_definitions(&arm.body, source, defs);
            }
        }
        Expr::Do(_, exprs) => {
            for e in exprs {
                collect_expr_definitions(e, source, defs);
            }
        }
        Expr::Computation(_, _, steps) => {
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, pattern, expr) => {
                        collect_pattern_definitions(pattern, source, defs);
                        collect_expr_definitions(expr, source, defs);
                    }
                    ComputationStep::DoBang(_, expr)
                    | ComputationStep::Return(_, expr)
                    | ComputationStep::Expr(expr) => {
                        collect_expr_definitions(expr, source, defs);
                    }
                }
            }
        }
        Expr::Unquote(_, expr) | Expr::UnquoteSplice(_, expr) => {
            collect_expr_definitions(expr, source, defs);
        }
        Expr::Quote(_, _) => {}
        _ => {}
    }
}

/// パターンからシンボル定義を収集する
fn collect_pattern_definitions(pat: &Pattern, source: &str, defs: &mut Vec<SymbolDef>) {
    match pat {
        Pattern::Var(span, name) => {
            let name_span = symbol_span_in_source(source, *span, name);
            defs.push(SymbolDef {
                name: name.clone(),
                start: name_span.start,
                end: name_span.end,
            });
        }
        Pattern::Constructor(_, _, fields) => {
            for f in fields {
                collect_pattern_definitions(f, source, defs);
            }
        }
        Pattern::RecordPat(_, _, fields) => {
            for (_, p) in fields {
                collect_pattern_definitions(p, source, defs);
            }
        }
        _ => {}
    }
}

/// AST からシンボル使用 (Expr::Var) を収集する
pub(crate) fn collect_usages(program: &Program) -> Vec<SymbolUsage> {
    let mut usages = Vec::new();
    for decl in &program.decls {
        collect_decl_usages(decl, &mut usages);
    }
    usages
}

/// 宣言からシンボル使用を収集する
fn collect_decl_usages(decl: &Decl, usages: &mut Vec<SymbolUsage>) {
    match decl {
        Decl::Defn { body, .. } => {
            collect_expr_usages(body, usages);
        }
        Decl::Private { inner, .. } => {
            collect_decl_usages(inner, usages);
        }
        _ => {}
    }
}

/// 式からシンボル使用を収集する
fn collect_expr_usages(expr: &Expr, usages: &mut Vec<SymbolUsage>) {
    match expr {
        Expr::Var(span, name) => {
            usages.push(SymbolUsage {
                name: name.clone(),
                start: span.start,
                end: span.end,
            });
        }
        Expr::Let(_, bindings, body) => {
            for (_, val) in bindings {
                collect_expr_usages(val, usages);
            }
            collect_expr_usages(body, usages);
        }
        Expr::If(_, cond, then_br, else_br) => {
            collect_expr_usages(cond, usages);
            collect_expr_usages(then_br, usages);
            collect_expr_usages(else_br, usages);
        }
        Expr::App(_, func, args) => {
            collect_expr_usages(func, usages);
            for arg in args {
                collect_expr_usages(arg, usages);
            }
        }
        Expr::Ann(_, expr, _) => {
            collect_expr_usages(expr, usages);
        }
        Expr::RecordLit(_, _, fields) => {
            for (_, value) in fields {
                collect_expr_usages(value, usages);
            }
        }
        Expr::FieldAccess(_, expr, _) => {
            collect_expr_usages(expr, usages);
        }
        Expr::RecordUpdate(_, expr, fields) => {
            collect_expr_usages(expr, usages);
            for (_, value) in fields {
                collect_expr_usages(value, usages);
            }
        }
        Expr::Lambda(_, _, body) => {
            collect_expr_usages(body, usages);
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_expr_usages(scrutinee, usages);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_expr_usages(guard, usages);
                }
                collect_expr_usages(&arm.body, usages);
            }
        }
        Expr::Do(_, exprs) => {
            for e in exprs {
                collect_expr_usages(e, usages);
            }
        }
        Expr::Computation(_, _, steps) => {
            for step in steps {
                match step {
                    ComputationStep::LetBang(_, _, expr)
                    | ComputationStep::DoBang(_, expr)
                    | ComputationStep::Return(_, expr)
                    | ComputationStep::Expr(expr) => {
                        collect_expr_usages(expr, usages);
                    }
                }
            }
        }
        Expr::Unquote(_, expr) | Expr::UnquoteSplice(_, expr) => {
            collect_expr_usages(expr, usages);
        }
        Expr::Quote(_, _) => {}
        _ => {}
    }
}

/// ソースコード内の指定位置にあるシンボルの定義位置を検索する
pub fn find_definition(source: &str, position: Position) -> Option<Range> {
    let offset = position_to_offset(source, position)?;
    let symbol_name = symbol_at_position(source, offset)?;
    let program = lsharp_syntax::parse(source).ok()?;
    let definitions = collect_definitions(&program, source);

    for def in &definitions {
        if def.name == symbol_name {
            let start = offset_to_position(source, def.start);
            let end = offset_to_position(source, def.end);
            return Some(Range::new(start, end));
        }
    }

    None
}

fn span_to_range(source: &str, span: Span) -> Range {
    let start = offset_to_position(source, span.start.min(source.len()));
    let end = offset_to_position(source, span.end.min(source.len()));
    Range::new(start, end)
}

fn diagnostic_error_at(
    source: &str,
    message: String,
    code: Option<&str>,
    span: Option<Span>,
) -> Diagnostic {
    Diagnostic {
        range: span
            .map(|span| span_to_range(source, span))
            .unwrap_or_default(),
        severity: Some(DiagnosticSeverity::ERROR),
        message,
        source: Some("lsharp".to_string()),
        code: code.map(|code| NumberOrString::String(code.to_string())),
        ..Default::default()
    }
}

fn diagnostic_error_from_message(source: &str, message: String) -> Diagnostic {
    let code = stable_code_from_message(&message).map(str::to_owned);
    let span = code
        .as_deref()
        .and_then(|code| diagnostic_span_from_message(source, code, &message));
    diagnostic_error_at(source, message, code.as_deref(), span)
}

fn stable_code_from_message(message: &str) -> Option<&str> {
    let bytes = message.as_bytes();
    for start in 0..bytes.len().saturating_sub(7) {
        if bytes[start] == b'['
            && bytes[start + 1] == b'L'
            && bytes[start + 2] == b'S'
            && bytes[start + 3..start + 7].iter().all(u8::is_ascii_digit)
            && bytes[start + 7] == b']'
        {
            return message.get(start + 1..start + 7);
        }
    }
    None
}

fn diagnostic_span_from_message(source: &str, code: &str, message: &str) -> Option<Span> {
    if code != "LS3102" {
        return None;
    }

    let prefix = "モジュール '";
    let suffix = "' が見つかりません";
    let module_start = message.find(prefix)? + prefix.len();
    let module_end = module_start + message[module_start..].find(suffix)?;
    let missing_module = &message[module_start..module_end];
    let program = lsharp_syntax::parse(source).ok()?;

    program.decls.iter().find_map(|decl| match decl {
        Decl::ImportDecl { span, module, .. } if module == missing_module => Some(*span),
        _ => None,
    })
}

fn parse_program(source: &str) -> std::result::Result<Program, Box<Diagnostic>> {
    lsharp_syntax::parse(source).map_err(|error| {
        let message = error.to_string();
        let code = error.code();
        let span = error.span();
        Box::new(diagnostic_error_at(source, message, Some(code), span))
    })
}

pub(crate) fn module_name_from_source(source: &str, path: &std::path::Path) -> String {
    if let Ok(program) = lsharp_syntax::parse(source) {
        for decl in &program.decls {
            if let Decl::ModuleDecl { name, .. } = decl {
                return name.clone();
            }
        }
    }

    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Main");
    stem.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(ch) => {
                    let mut result = ch.to_uppercase().to_string();
                    result.extend(chars);
                    result
                }
            }
        })
        .collect()
}

pub(crate) fn imported_modules_from_source(source: &str) -> Vec<String> {
    if let Ok(program) = lsharp_syntax::parse(source) {
        return program
            .decls
            .iter()
            .filter_map(|decl| match decl {
                Decl::ImportDecl { module, .. } => Some(module.clone()),
                _ => None,
            })
            .collect();
    }

    Vec::new()
}

/// ソースコードをパースし、syntax error があれば診断情報を返す
pub(crate) fn parse_only(source: &str) -> Vec<Diagnostic> {
    match parse_program(source) {
        Ok(_) => Vec::new(),
        Err(diagnostic) => vec![*diagnostic],
    }
}

/// ソースコードをパース・型チェックし、診断情報を返す
pub fn parse_and_check(source: &str) -> Vec<Diagnostic> {
    let program = match parse_program(source) {
        Ok(program) => program,
        Err(diagnostic) => {
            return vec![*diagnostic];
        }
    };
    let mut diagnostics = Vec::new();

    let mut infer = lsharp_types::infer::Infer::new();
    if let Err(e) = infer.infer_program(&program) {
        let message = e.to_string();
        let code = e.code();
        let span = e.span();
        diagnostics.push(diagnostic_error_at(source, message, Some(code), span));
    }

    diagnostics
}

pub(crate) fn parse_and_check_incremental(
    module_key: &str,
    source: &str,
    cache: &mut lsharp_ir::CompilationCache,
) -> Vec<Diagnostic> {
    match lsharp_ir::analyze_single_file_incremental(module_key, source, cache) {
        Ok(()) => Vec::new(),
        Err(message) => {
            let diagnostics = parse_and_check(source);
            if diagnostics.is_empty() {
                vec![diagnostic_error_from_message(source, message)]
            } else {
                diagnostics
            }
        }
    }
}

pub(crate) fn parse_and_check_uri_incremental(
    uri: &Url,
    source: &str,
    source_overrides: &std::collections::HashMap<std::path::PathBuf, String>,
    cache: &mut lsharp_ir::CompilationCache,
) -> Vec<Diagnostic> {
    if uri.scheme() == "file"
        && let Ok(path) = uri.to_file_path()
    {
        let mut overrides = source_overrides.clone();
        overrides.insert(path.clone(), source.to_string());
        return match lsharp_ir::analyze_multi_file_incremental_with_overrides(
            &path, &overrides, cache,
        ) {
            Ok(()) => Vec::new(),
            Err(message) => vec![diagnostic_error_from_message(source, message)],
        };
    }

    parse_and_check_incremental(uri.as_ref(), source, cache)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_to_position_basic() {
        let source = "hello\nworld";
        assert_eq!(offset_to_position(source, 0), Position::new(0, 0));
        assert_eq!(offset_to_position(source, 5), Position::new(0, 5));
        assert_eq!(offset_to_position(source, 6), Position::new(1, 0));
        assert_eq!(offset_to_position(source, 8), Position::new(1, 2));
    }

    #[test]
    fn test_position_to_offset_basic() {
        let source = "hello\nworld";
        assert_eq!(position_to_offset(source, Position::new(0, 0)), Some(0));
        assert_eq!(position_to_offset(source, Position::new(1, 0)), Some(6));
        assert_eq!(position_to_offset(source, Position::new(1, 2)), Some(8));
    }

    #[test]
    fn positions_use_utf16_code_units_for_non_ascii_source() {
        let source = "😀 x\n日本語";

        assert_eq!(offset_to_position(source, 1), Position::new(0, 0));
        assert_eq!(offset_to_position(source, 4), Position::new(0, 2));
        assert_eq!(offset_to_position(source, 6), Position::new(0, 4));
        assert_eq!(offset_to_position(source, 16), Position::new(1, 3));

        assert_eq!(position_to_offset(source, Position::new(0, 1)), Some(0));
        assert_eq!(position_to_offset(source, Position::new(0, 2)), Some(4));
        assert_eq!(position_to_offset(source, Position::new(0, 3)), Some(5));
        assert_eq!(position_to_offset(source, Position::new(1, 3)), Some(16));
    }

    #[test]
    fn test_symbol_at_position_basic() {
        let source = "(defn add [x y] (+ x y))";
        // "add" は offset 6 から
        assert_eq!(symbol_at_position(source, 6), Some("add".to_string()));
        // "(" はシンボルではない
        assert_eq!(symbol_at_position(source, 0), None);
    }

    #[test]
    fn test_symbol_range_at_position_basic() {
        let source = "(defn add [x y] (+ x y))";
        let result = symbol_range_at_position(source, 6);
        assert!(result.is_some());
        let (name, start, end) = result.unwrap();
        assert_eq!(name, "add");
        assert_eq!(&source[start..end], "add");
    }

    #[test]
    fn test_collect_usages_basic() {
        let source = "(defn f [x] (+ x x))";
        let program = lsharp_syntax::parse(source).unwrap();
        let usages = collect_usages(&program);
        // "+" と "x" の使用が収集される
        let x_usages: Vec<_> = usages.iter().filter(|u| u.name == "x").collect();
        assert_eq!(x_usages.len(), 2, "x は 2 箇所で使用されるべき");
    }

    #[test]
    fn syntax_diagnostics_expose_stable_code_and_source_range() {
        let diagnostics = parse_only("(unknown-form)");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("LS0103".to_string()))
        );
        assert_eq!(
            diagnostics[0].range,
            Range::new(Position::new(0, 1), Position::new(0, 13))
        );
    }

    #[test]
    fn type_diagnostics_expose_stable_code_and_non_empty_source_range() {
        let diagnostics = parse_and_check("(defn bad [] (+ 1 true))");
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("LS1004".to_string()))
        );
        assert_ne!(diagnostics[0].range, Range::default());
    }

    #[test]
    fn incremental_type_diagnostics_forward_stable_code_and_source_range() {
        let mut cache = lsharp_ir::CompilationCache::new();
        let diagnostics =
            parse_and_check_incremental("Main", "(defn bad [] (+ 1 true))", &mut cache);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("LS1004".to_string()))
        );
        assert_ne!(diagnostics[0].range, Range::default());
    }

    #[test]
    fn incremental_module_diagnostics_forward_stable_code() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_lsp_incremental_module_diagnostic_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock は unix epoch より後であるべき")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("Main.ls");
        let source = "(module Main)\n(import Missing)\n(defn main [] 1)\n";
        std::fs::write(&entry, source).unwrap();

        let uri = Url::from_file_path(&entry).expect("entry path は file URI へ変換できるべき");
        let mut overrides = HashMap::new();
        overrides.insert(entry.clone(), source.to_string());
        let mut cache = lsharp_ir::CompilationCache::new();
        let diagnostics = parse_and_check_uri_incremental(&uri, source, &overrides, &mut cache);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("LS3102".to_string()))
        );
        assert!(diagnostics[0].message.contains("Missing"));
        assert_eq!(
            diagnostics[0].range,
            Range::new(Position::new(1, 0), Position::new(1, 16))
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn incremental_module_type_diagnostics_forward_stable_code() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_lsp_incremental_module_type_diagnostic_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock は unix epoch より後であるべき")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("Main.ls");
        let helper = dir.join("Helpers.ls");
        let main_source = "(module Main)\n(import Helpers)\n(defn main [] (+ (helper) 1))\n";
        std::fs::write(&entry, main_source).unwrap();
        std::fs::write(&helper, "(module Helpers)\n(defn helper [] true)\n").unwrap();

        let uri = Url::from_file_path(&entry).expect("entry path は file URI へ変換できるべき");
        let mut overrides = HashMap::new();
        overrides.insert(entry.clone(), main_source.to_string());
        let mut cache = lsharp_ir::CompilationCache::new();
        let diagnostics =
            parse_and_check_uri_incremental(&uri, main_source, &overrides, &mut cache);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("LS1004".to_string()))
        );
        assert!(diagnostics[0].message.contains("Main.ls"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn incremental_module_parse_diagnostics_forward_stable_code() {
        use std::collections::HashMap;

        let dir = std::env::temp_dir().join(format!(
            "lsharp_lsp_incremental_module_parse_diagnostic_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock は unix epoch より後であるべき")
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.join("Main.ls");
        let helper = dir.join("Helpers.ls");
        let main_source = "(module Main)\n(import Helpers)\n(defn main []";
        std::fs::write(&entry, main_source).unwrap();
        std::fs::write(&helper, "(module Helpers)\n(defn helper [] 1)\n").unwrap();

        let uri = Url::from_file_path(&entry).expect("entry path は file URI へ変換できるべき");
        let mut overrides = HashMap::new();
        overrides.insert(entry.clone(), main_source.to_string());
        let mut cache = lsharp_ir::CompilationCache::new();
        let diagnostics =
            parse_and_check_uri_incremental(&uri, main_source, &overrides, &mut cache);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            Some(NumberOrString::String("LS0101".to_string()))
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
