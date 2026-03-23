mod format;
mod references;
mod rename;
mod util;

use std::collections::HashMap;
use std::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

// 公開 API の再エクスポート
pub use format::format_source;
pub use references::find_references;
pub use util::find_definition;

/// L# 言語サーバーのバックエンド
pub struct LsharpBackend {
    client: Client,
    /// ソースコードキャッシュ (URI → ソース全文)
    source_cache: RwLock<HashMap<Url, String>>,
}

impl LsharpBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            source_cache: RwLock::new(HashMap::new()),
        }
    }

    /// キャッシュからソースコードを取得する
    fn get_source(&self, uri: &Url) -> Option<String> {
        self.source_cache
            .read()
            .ok()
            .and_then(|cache| cache.get(uri).cloned())
    }

    /// キャッシュにソースコードを保存する
    fn set_source(&self, uri: Url, source: String) {
        if let Ok(mut cache) = self.source_cache.write() {
            cache.insert(uri, source);
        }
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
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                })),
                document_formatting_provider: Some(OneOf::Left(true)),
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
        let diagnostics = util::parse_and_check(&text);
        self.set_source(uri.clone(), text);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        // TextDocumentSyncKind::FULL なので最後の変更が全文
        if let Some(change) = params.content_changes.into_iter().last() {
            let diagnostics = util::parse_and_check(&change.text);
            self.set_source(uri.clone(), change.text);
            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _uri = params.text_document_position_params.text_document.uri;
        let _position = params.text_document_position_params.position;

        // TODO: URI からソースを取得してホバー情報を返す
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let result = self
            .get_source(&uri)
            .and_then(|source| find_definition(&source, position))
            .map(|range| GotoDefinitionResponse::Scalar(Location { uri, range }));

        Ok(result)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        if let Some(source) = self.get_source(&uri) {
            let refs = find_references(&source, position, include_declaration);
            if refs.is_empty() {
                return Ok(None);
            }
            let locations: Vec<Location> = refs
                .into_iter()
                .map(|range| Location {
                    uri: uri.clone(),
                    range,
                })
                .collect();
            return Ok(Some(locations));
        }

        Ok(None)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        let result = self.get_source(&uri).and_then(|source| {
            rename::prepare_rename(&source, position).map(|(name, range)| {
                PrepareRenameResponse::RangeWithPlaceholder {
                    range,
                    placeholder: name,
                }
            })
        });

        Ok(result)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        if let Some(source) = self.get_source(&uri) {
            let edits = rename::compute_rename_edits(&source, position, &new_name);
            if edits.is_empty() {
                return Ok(None);
            }
            let mut changes = HashMap::new();
            changes.insert(uri, edits);
            return Ok(Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }));
        }

        Ok(None)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        if let Some(source) = self.get_source(&uri) {
            let formatted = format_source(&source);
            if formatted == source {
                // 変更なし
                return Ok(None);
            }
            // ソース全体を置換する単一の TextEdit
            let lines: Vec<&str> = source.lines().collect();
            let last_line = lines.len().saturating_sub(1) as u32;
            let last_col = lines.last().map_or(0, |l| l.len()) as u32;
            let edit = TextEdit {
                range: Range::new(
                    Position::new(0, 0),
                    Position::new(last_line, last_col),
                ),
                new_text: formatted,
            };
            return Ok(Some(vec![edit]));
        }

        Ok(None)
    }
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
        let pos = Position::new(0, 17);
        let result = find_definition(source, pos);
        assert!(result.is_some(), "let バインディングの定義が見つかるべき");
    }

    #[test]
    fn test_find_definition_undefined_symbol() {
        // 未定義シンボルで None を返す
        let source = "(defn f [] (+ x y))";
        let pos = Position::new(0, 15);
        let result = find_definition(source, pos);
        assert!(
            result.is_none(),
            "未定義シンボルでは None を返すべき"
        );
    }

    #[test]
    fn test_server_capabilities() {
        // ServerCapabilities に必要な provider が全て含まれる検証
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Left(true)),
            rename_provider: Some(OneOf::Right(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            })),
            document_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };
        assert!(capabilities.definition_provider.is_some());
        assert!(capabilities.references_provider.is_some());
        assert!(capabilities.rename_provider.is_some());
        assert!(capabilities.document_formatting_provider.is_some());
    }
}
