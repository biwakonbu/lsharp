use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

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

    let (service, socket) = tower_lsp::LspService::new(|client| LsharpBackend::new(client));
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
