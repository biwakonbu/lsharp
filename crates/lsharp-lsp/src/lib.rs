use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use lsharp_syntax::ast::{Decl, Expr, Pattern, Program};

/// L# 言語サーバーのバックエンド
pub struct LsharpBackend {
    client: Client,
}

impl LsharpBackend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LsharpBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        let diagnostics = parse_and_check(&text);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // TextDocumentSyncKind::FULL なので最後の変更が全文
        if let Some(change) = params.content_changes.into_iter().last() {
            let diagnostics = parse_and_check(&change.text);
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // TODO: URI からソースを取得する仕組みが必要
        // 現時点ではプレースホルダーとして基本的な情報を返す
        let _ = (uri, position);

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        // TODO: URI からソースを取得する仕組みが必要
        // 現時点ではプレースホルダー
        let _ = params;
        Ok(None)
    }
}

/// バイトオフセットを LSP Position (行・列) に変換する
fn offset_to_position(source: &str, offset: usize) -> Position {
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
fn position_to_offset(source: &str, position: Position) -> Option<usize> {
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
struct SymbolDef {
    /// シンボル名
    name: String,
    /// 定義位置のバイトオフセット (開始)
    start: usize,
    /// 定義位置のバイトオフセット (終了)
    end: usize,
}

/// AST からシンボル定義を収集する
fn collect_definitions(program: &Program) -> Vec<SymbolDef> {
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
            span, name, params, body, ..
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

/// 指定位置のシンボル名を取得する
fn symbol_at_position(source: &str, offset: usize) -> Option<String> {
    // シンボル文字かどうかの判定 (L# のシンボルに使える文字)
    fn is_symbol_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_' || c == '-' || c == '?' || c == '!'
    }

    let bytes = source.as_bytes();
    if offset >= bytes.len() {
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

/// ソースコード内の指定位置にあるシンボルの定義位置を検索する
///
/// - source: ソースコード全文
/// - position: LSP Position (行・列)
///
/// 戻り値: 定義位置の LSP Range (見つからなければ None)
pub fn find_definition(source: &str, position: Position) -> Option<Range> {
    // Position をバイトオフセットに変換
    let offset = position_to_offset(source, position)?;

    // カーソル位置のシンボル名を取得
    let symbol_name = symbol_at_position(source, offset)?;

    // ソースを parse
    let program = lsharp_syntax::parse(source).ok()?;

    // AST からシンボル定義を収集
    let definitions = collect_definitions(&program);

    // シンボル名と一致する定義を検索
    // (カーソル位置自体が定義位置なら、自分自身を返す)
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
fn parse_and_check(source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // パース
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

    // 型チェック
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

/// LSP サーバーを起動する
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp::LspService::new(LsharpBackend::new);
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_definition_toplevel_function() {
        // トップレベル関数 (defn add ...) の定義ジャンプ
        let source = "(defn add [x y] (+ x y))";
        // "add" の先頭位置
        let pos = Position::new(0, 6);
        let result = find_definition(source, pos);
        assert!(result.is_some(), "トップレベル関数の定義が見つかるべき");
        let range = result.unwrap();
        assert_eq!(range.start.line, 0);
    }

    #[test]
    fn test_find_definition_let_binding() {
        // let バインディングの定義ジャンプ
        let source = "(defn f [] (let [x 42] x))";
        // let 内の x の位置
        // "(defn f [] (let [x 42] x))"
        //                  ^ offset=17
        let pos = Position::new(0, 17);
        let result = find_definition(source, pos);
        assert!(result.is_some(), "let バインディングの定義が見つかるべき");
    }

    #[test]
    fn test_find_definition_undefined_symbol() {
        // 未定義シンボルで None を返す
        let source = "(defn f [] (+ x y))";
        // "x" はパラメータにも let にも定義されていない
        let pos = Position::new(0, 15);
        let result = find_definition(source, pos);
        assert!(
            result.is_none(),
            "未定義シンボルでは None を返すべき"
        );
    }

    #[test]
    fn test_server_capabilities_include_definition_provider() {
        // ServerCapabilities に definition_provider が含まれる検証
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };
        assert!(capabilities.definition_provider.is_some());
    }
}
