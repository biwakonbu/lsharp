use tower_lsp::lsp_types::*;

use lsharp_syntax::ast::{Decl, Expr, Pattern, Program};

/// バイトオフセットを LSP Position (行・列) に変換する
pub(crate) fn offset_to_position(source: &str, offset: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
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
            col += 1;
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

/// 指定位置のシンボル名を取得する
pub(crate) fn symbol_at_position(source: &str, offset: usize) -> Option<String> {
    fn is_symbol_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '?' || c == '!'
    }

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
    fn is_symbol_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '?' || c == '!'
    }

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
pub(crate) fn collect_definitions(program: &Program) -> Vec<SymbolDef> {
    let mut defs = Vec::new();
    for decl in &program.decls {
        collect_decl_definitions(decl, &mut defs);
    }
    defs
}

/// 宣言からシンボル定義を収集する
fn collect_decl_definitions(decl: &Decl, defs: &mut Vec<SymbolDef>) {
    match decl {
        Decl::Defn {
            span,
            name,
            params,
            body,
            ..
        } => {
            // 関数名の定義位置
            defs.push(SymbolDef {
                name: name.clone(),
                start: span.start,
                end: span.end,
            });
            // パラメータの定義位置
            for param in params {
                defs.push(SymbolDef {
                    name: param.name.clone(),
                    start: param.span.start,
                    end: param.span.end,
                });
            }
            // body 内の let バインディングを収集
            collect_expr_definitions(body, defs);
        }
        Decl::Private { inner, .. } => {
            collect_decl_definitions(inner, defs);
        }
        _ => {}
    }
}

/// 式内の let バインディング等からシンボル定義を収集する
fn collect_expr_definitions(expr: &Expr, defs: &mut Vec<SymbolDef>) {
    match expr {
        Expr::Let(_, bindings, body) => {
            for (pat, val) in bindings {
                collect_pattern_definitions(pat, defs);
                collect_expr_definitions(val, defs);
            }
            collect_expr_definitions(body, defs);
        }
        Expr::If(_, cond, then_br, else_br) => {
            collect_expr_definitions(cond, defs);
            collect_expr_definitions(then_br, defs);
            collect_expr_definitions(else_br, defs);
        }
        Expr::App(_, func, args) => {
            collect_expr_definitions(func, defs);
            for arg in args {
                collect_expr_definitions(arg, defs);
            }
        }
        Expr::Lambda(_, params, body) => {
            for param in params {
                defs.push(SymbolDef {
                    name: param.name.clone(),
                    start: param.span.start,
                    end: param.span.end,
                });
            }
            collect_expr_definitions(body, defs);
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_expr_definitions(scrutinee, defs);
            for arm in arms {
                collect_pattern_definitions(&arm.pattern, defs);
                collect_expr_definitions(&arm.body, defs);
            }
        }
        Expr::Do(_, exprs) => {
            for e in exprs {
                collect_expr_definitions(e, defs);
            }
        }
        _ => {}
    }
}

/// パターンからシンボル定義を収集する
fn collect_pattern_definitions(pat: &Pattern, defs: &mut Vec<SymbolDef>) {
    match pat {
        Pattern::Var(span, name) => {
            defs.push(SymbolDef {
                name: name.clone(),
                start: span.start,
                end: span.end,
            });
        }
        Pattern::Constructor(_, _, fields) => {
            for f in fields {
                collect_pattern_definitions(f, defs);
            }
        }
        Pattern::RecordPat(_, _, fields) => {
            for (_, p) in fields {
                collect_pattern_definitions(p, defs);
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
        Expr::Lambda(_, _, body) => {
            collect_expr_usages(body, usages);
        }
        Expr::Match(_, scrutinee, arms) => {
            collect_expr_usages(scrutinee, usages);
            for arm in arms {
                collect_expr_usages(&arm.body, usages);
            }
        }
        Expr::Do(_, exprs) => {
            for e in exprs {
                collect_expr_usages(e, usages);
            }
        }
        _ => {}
    }
}

/// ソースコード内の指定位置にあるシンボルの定義位置を検索する
pub fn find_definition(source: &str, position: Position) -> Option<Range> {
    let offset = position_to_offset(source, position)?;
    let symbol_name = symbol_at_position(source, offset)?;
    let program = lsharp_syntax::parse(source).ok()?;
    let definitions = collect_definitions(&program);

    for def in &definitions {
        if def.name == symbol_name {
            let start = offset_to_position(source, def.start);
            let end = offset_to_position(source, def.end);
            return Some(Range::new(start, end));
        }
    }

    None
}

/// ソースコードをパース・型チェックし、診断情報を返す
pub fn parse_and_check(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let program = match lsharp_syntax::parse(source) {
        Ok(p) => p,
        Err(e) => {
            diagnostics.push(Diagnostic {
                range: Range::new(Position::new(0, 0), Position::new(0, 0)),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("{e}"),
                source: Some("lsharp".to_string()),
                ..Default::default()
            });
            return diagnostics;
        }
    };

    let mut infer = lsharp_types::infer::Infer::new();
    if let Err(e) = infer.infer_program(&program) {
        diagnostics.push(Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            severity: Some(DiagnosticSeverity::ERROR),
            message: format!("{e}"),
            source: Some("lsharp".to_string()),
            ..Default::default()
        });
    }

    diagnostics
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
    fn test_symbol_at_position_basic() {
        let source = "(defn add [x y] (+ x y))";
        // "add" は offset 6 から
        assert_eq!(
            symbol_at_position(source, 6),
            Some("add".to_string())
        );
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
}
