mod analysis;
mod completion;
mod format;
mod references;
mod rename;
mod text_sync;
mod util;

use std::collections::HashMap;
use std::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

// 公開 API の再エクスポート
pub use analysis::hover as analyze_hover;
pub use completion::complete as analyze_completion;
pub use format::format_source;
pub use references::find_references;
pub use tower_lsp::lsp_types::{
    CompletionItem, Hover, HoverContents, MarkedString, Position, Range,
};
pub use util::find_definition;
pub use util::parse_and_check;

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

fn text_document_sync_kind() -> TextDocumentSyncKind {
    TextDocumentSyncKind::INCREMENTAL
}

#[tower_lsp::async_trait]
impl LanguageServer for LsharpBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    text_document_sync_kind(),
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions::default()),
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
        let current_source = self.get_source(&uri).unwrap_or_default();
        let updated_source =
            text_sync::apply_content_changes(&current_source, &params.content_changes)
                .unwrap_or(current_source);
        let diagnostics = util::parse_and_check(&updated_source);
        self.set_source(uri.clone(), updated_source);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        Ok(self
            .get_source(&uri)
            .and_then(|source| analysis::hover(&source, position)))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let items = self
            .get_source(&uri)
            .map(|source| completion::complete(&source, position, &[]))
            .unwrap_or_default();
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
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
                range: Range::new(Position::new(0, 0), Position::new(last_line, last_col)),
                new_text: formatted,
            };
            return Ok(Some(vec![edit]));
        }

        Ok(None)
    }
}

/// tower-lsp の `FromParams for ()` が `params: null` や `params: {}` を拒否する問題を
/// 回避するミドルウェア。`shutdown` 等のパラメータなしメソッドで `null` や空オブジェクトが
/// 送られた場合、params を除去してから内部サービスへ転送する。
mod params_normalizer {
    use serde_json::Value;
    use std::task::{Context, Poll};
    use tower_lsp::jsonrpc::{Request, Response};

    /// パラメータなしの LSP メソッド一覧
    const PARAMLESS_METHODS: &[&str] = &["shutdown"];

    /// params が意味的に空かどうかを判定する
    fn is_empty_params(v: &Value) -> bool {
        v.is_null() || (v.is_object() && v.as_object().unwrap().is_empty())
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use serde_json::json;

        #[test]
        fn test_is_empty_params_null() {
            assert!(is_empty_params(&Value::Null));
        }

        #[test]
        fn test_is_empty_params_empty_object() {
            assert!(is_empty_params(&json!({})));
        }

        #[test]
        fn test_is_empty_params_non_empty_object() {
            assert!(!is_empty_params(&json!({"key": "value"})));
        }

        #[test]
        fn test_is_empty_params_number() {
            assert!(!is_empty_params(&json!(42)));
        }

        #[test]
        fn test_shutdown_request_params_null_stripped() {
            // params: null の shutdown リクエストから params が除去されることを確認
            let req = Request::build("shutdown")
                .params(Value::Null)
                .id(1)
                .finish();
            assert!(req.params().is_some());

            let needs_strip = PARAMLESS_METHODS.contains(&req.method())
                && req.params().is_some_and(is_empty_params);
            assert!(needs_strip);

            let (method, id, _) = req.into_parts();
            let mut builder = Request::build(method);
            if let Some(id) = id {
                builder = builder.id(id);
            }
            let stripped = builder.finish();
            assert!(stripped.params().is_none());
            assert_eq!(stripped.method(), "shutdown");
        }

        #[test]
        fn test_shutdown_request_params_empty_object_stripped() {
            let req = Request::build("shutdown").params(json!({})).id(2).finish();
            let needs_strip = PARAMLESS_METHODS.contains(&req.method())
                && req.params().is_some_and(is_empty_params);
            assert!(needs_strip);
        }

        #[test]
        fn test_non_shutdown_method_not_stripped() {
            let req = Request::build("textDocument/hover")
                .params(json!({"textDocument": {"uri": "file:///test.ls"}}))
                .id(3)
                .finish();
            let needs_strip = PARAMLESS_METHODS.contains(&req.method())
                && req.params().is_some_and(is_empty_params);
            assert!(!needs_strip);
        }
    }

    pub struct ParamsNormalizer<S> {
        inner: S,
    }

    impl<S> ParamsNormalizer<S> {
        pub fn new(inner: S) -> Self {
            Self { inner }
        }
    }

    impl<S> tower_service::Service<Request> for ParamsNormalizer<S>
    where
        S: tower_service::Service<Request, Response = Option<Response>>,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(
            &mut self,
            cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, req: Request) -> Self::Future {
            let needs_strip = PARAMLESS_METHODS.contains(&req.method())
                && req.params().is_some_and(is_empty_params);

            if needs_strip {
                // params を除去した新しい Request を構築
                let (method, id, _params) = req.into_parts();
                let mut builder = Request::build(method);
                if let Some(id) = id {
                    builder = builder.id(id);
                }
                self.inner.call(builder.finish())
            } else {
                self.inner.call(req)
            }
        }
    }
}

/// LSP サーバーを起動する
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = tower_lsp::LspService::new(LsharpBackend::new);
    let service = params_normalizer::ParamsNormalizer::new(service);
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
        assert!(result.is_none(), "未定義シンボルでは None を返すべき");
    }

    #[test]
    fn test_server_capabilities() {
        // ServerCapabilities に必要な provider が全て含まれる検証
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(text_document_sync_kind())),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions::default()),
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
        assert!(capabilities.hover_provider.is_some());
        assert!(capabilities.completion_provider.is_some());
        assert!(capabilities.definition_provider.is_some());
        assert!(capabilities.references_provider.is_some());
        assert!(capabilities.rename_provider.is_some());
        assert!(capabilities.document_formatting_provider.is_some());
    }

    #[test]
    fn test_text_document_sync_kind_is_incremental() {
        assert_eq!(
            text_document_sync_kind(),
            TextDocumentSyncKind::INCREMENTAL,
            "INC-F4 では LSP sync kind を INCREMENTAL へ切り替えるべき"
        );
    }
}
