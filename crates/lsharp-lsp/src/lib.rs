mod analysis;
mod completion;
mod format;
mod references;
mod rename;
mod text_sync;
mod util;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use lsharp_ir::CompilationCache;
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
    source_cache: Arc<RwLock<HashMap<Url, String>>>,
    /// document version cache (URI → 最終 version)
    version_cache: Arc<RwLock<HashMap<Url, i32>>>,
    /// Rust LSP 向け single-file incremental compile cache
    compilation_cache: Arc<RwLock<CompilationCache>>,
}

impl LsharpBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            source_cache: Arc::new(RwLock::new(HashMap::new())),
            version_cache: Arc::new(RwLock::new(HashMap::new())),
            compilation_cache: Arc::new(RwLock::new(CompilationCache::new())),
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
    fn set_source(&self, uri: Url, source: String, version: i32) {
        if let Ok(mut cache) = self.source_cache.write() {
            cache.insert(uri.clone(), source);
        }
        if let Ok(mut versions) = self.version_cache.write() {
            versions.insert(uri, version);
        }
    }

    fn is_current_version(
        version_cache: &Arc<RwLock<HashMap<Url, i32>>>,
        uri: &Url,
        version: i32,
    ) -> bool {
        version_cache
            .read()
            .ok()
            .and_then(|versions| versions.get(uri).copied())
            == Some(version)
    }

    fn file_source_overrides(&self) -> HashMap<std::path::PathBuf, String> {
        self.source_cache
            .read()
            .ok()
            .map(|cache| {
                cache
                    .iter()
                    .filter_map(|(uri, source)| {
                        (uri.scheme() == "file")
                            .then(|| uri.to_file_path().ok().map(|path| (path, source.clone())))
                            .flatten()
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn publish_fast_diagnostics(&self, uri: Url, source: String, version: i32) {
        self.set_source(uri.clone(), source.clone(), version);

        let diagnostics = util::parse_only(&source);
        self.client
            .publish_diagnostics(uri.clone(), diagnostics.clone(), Some(version))
            .await;

        if !diagnostics.is_empty() {
            return;
        }

        let client = self.client.clone();
        let version_cache = Arc::clone(&self.version_cache);
        let compilation_cache = Arc::clone(&self.compilation_cache);
        let uri_for_analysis = uri.clone();
        let source_overrides = self.file_source_overrides();
        tokio::spawn(async move {
            let full_diagnostics = tokio::task::spawn_blocking(move || {
                let mut cache = compilation_cache
                    .write()
                    .expect("compilation cache lock should be available");
                util::parse_and_check_uri_incremental(
                    &uri_for_analysis,
                    &source,
                    &source_overrides,
                    &mut cache,
                )
            })
            .await
            .expect("full diagnostics task should complete");

            if LsharpBackend::is_current_version(&version_cache, &uri, version) {
                client
                    .publish_diagnostics(uri, full_diagnostics, Some(version))
                    .await;
            }
        });
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
        let version = params.text_document.version;
        self.publish_fast_diagnostics(uri, text, version).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        let current_source = self.get_source(&uri).unwrap_or_default();
        let updated_source =
            text_sync::apply_content_changes(&current_source, &params.content_changes)
                .unwrap_or(current_source);
        self.publish_fast_diagnostics(uri, updated_source, version)
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
    use futures::StreamExt;
    use serde_json::{Value, json};
    use tower_lsp::jsonrpc::{Request, Response};
    use tower_service::Service;

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

    #[tokio::test]
    async fn test_incremental_did_change_publishes_diagnostics_under_50ms() {
        let (document_source, changed_line, replacement_start, replacement_end) =
            benchmark_document_fixture();
        let changed_uri = "file:///timing-test.ls";
        let change_text = "999";

        let (mut service, mut socket) = initialize_test_server().await;

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": changed_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": document_source,
                    }
                }
            }),
        )
        .await;
        let _ = read_publish_diagnostics(&mut socket).await;

        let start = std::time::Instant::now();
        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {
                        "uri": changed_uri,
                        "version": 2
                    },
                    "contentChanges": [{
                        "range": {
                            "start": { "line": changed_line, "character": replacement_start },
                            "end": { "line": changed_line, "character": replacement_end }
                        },
                        "text": change_text
                    }]
                }
            }),
        )
        .await;
        let diagnostics = read_publish_diagnostics(&mut socket).await;
        let elapsed = start.elapsed();

        assert_eq!(
            diagnostics["params"]["uri"].as_str(),
            Some(changed_uri),
            "didChange 後の diagnostics publish は同一 URI を返すべき"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "1000 行 document の didChange -> publishDiagnostics は 50ms 未満であるべき: {:?}",
            elapsed
        );
    }

    #[tokio::test]
    async fn test_did_open_eventually_publishes_type_diagnostics() {
        let (mut service, mut socket) = initialize_test_server().await;
        let changed_uri = "file:///type-error.ls";

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": changed_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": "(defn bad [] (+ 1 true))",
                    }
                }
            }),
        )
        .await;

        let fast = read_publish_diagnostics(&mut socket).await;
        assert_eq!(
            fast["params"]["uri"].as_str(),
            Some(changed_uri),
            "最初の diagnostics publish は対象 URI に向くべき"
        );
        assert_eq!(
            fast["params"]["version"].as_i64(),
            Some(1),
            "最初の diagnostics publish は open version を保持するべき"
        );
        assert_eq!(
            fast["params"]["diagnostics"]
                .as_array()
                .map(std::vec::Vec::len),
            Some(0),
            "syntax-only fast path は well-formed source で空 diagnostics を返すべき"
        );

        let full = read_publish_diagnostics(&mut socket).await;
        assert_eq!(
            full["params"]["version"].as_i64(),
            Some(1),
            "後段 full diagnostics も同じ version を保持するべき"
        );
        assert!(
            full["params"]["diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty()),
            "後段 full diagnostics は type error を報告するべき"
        );
    }

    #[tokio::test]
    async fn test_did_open_eventually_publishes_multi_file_import_diagnostics_from_unsaved_source()
    {
        let workspace = unique_temp_dir("lsharp_lsp_multifile_unsaved_import");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(
            workspace.join("Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] 1)\n",
        )
        .unwrap();
        std::fs::write(
            workspace.join("Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)\n",
        )
        .unwrap();

        let changed_uri = Url::from_file_path(workspace.join("Main.ls"))
            .expect("temp workspace path should convert to file url");
        let (mut service, mut socket) = initialize_test_server().await;

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": changed_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": "(module Main)\n(import Missing)\n(defn main [] 1)\n",
                    }
                }
            }),
        )
        .await;

        let fast = read_publish_diagnostics(&mut socket).await;
        assert_eq!(
            fast["params"]["diagnostics"]
                .as_array()
                .map(std::vec::Vec::len),
            Some(0),
            "syntax-only fast path は well-formed source で空 diagnostics を返すべき"
        );

        let full = read_publish_diagnostics(&mut socket).await;
        let diagnostics = full["params"]["diagnostics"]
            .as_array()
            .expect("full diagnostics payload should be an array");
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("Missing"))
            }),
            "multi-file diagnostics は unsaved source の missing import を報告するべき: {diagnostics:?}"
        );

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[tokio::test]
    async fn test_did_open_uses_unsaved_open_dependency_overlay_for_multi_file_diagnostics() {
        let workspace = unique_temp_dir("lsharp_lsp_open_dependency_overlay");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(
            workspace.join("Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();
        std::fs::write(
            workspace.join("Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)\n",
        )
        .unwrap();

        let main_uri = Url::from_file_path(workspace.join("Main.ls"))
            .expect("temp main path should convert to file url");
        let helpers_uri = Url::from_file_path(workspace.join("Helpers.ls"))
            .expect("temp helpers path should convert to file url");
        let (mut service, mut socket) = initialize_test_server().await;

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": helpers_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": "(module Helpers)\n(defn helper [] true)\n",
                    }
                }
            }),
        )
        .await;
        let _ = read_publish_diagnostics(&mut socket).await;
        let _ = read_publish_diagnostics(&mut socket).await;

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": main_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": "(module Main)\n(import Helpers)\n(defn main [] (+ (helper) 1))\n",
                    }
                }
            }),
        )
        .await;

        let fast = read_publish_diagnostics(&mut socket).await;
        assert_eq!(
            fast["params"]["diagnostics"]
                .as_array()
                .map(std::vec::Vec::len),
            Some(0),
            "syntax-only fast path は active file 単体で空 diagnostics を返すべき"
        );

        let full = read_publish_diagnostics(&mut socket).await;
        assert!(
            full["params"]["diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty()),
            "active file の full diagnostics は open 済み dependency の unsaved overlay を使うべき"
        );

        let _ = std::fs::remove_dir_all(&workspace);
    }

    fn benchmark_document_fixture() -> (String, u32, u32, u32) {
        let mut source = String::from("(module Main)\n");
        for idx in 0..1000 {
            source.push_str(&format!("(defn helper-{idx} [] {idx})\n"));
        }
        source.push_str("(defn main [] (helper-500))\n");

        let changed_line = 501u32;
        let target_line = format!("(defn helper-500 [] 500)");
        let replacement_start = target_line.find("500)").expect("literal start") as u32;
        let replacement_end = replacement_start + 3;

        (source, changed_line, replacement_start, replacement_end)
    }

    fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}_{unique}_{}", std::process::id()))
    }

    type TestService = params_normalizer::ParamsNormalizer<tower_lsp::LspService<LsharpBackend>>;
    type TestSocket = tower_lsp::ClientSocket;

    fn spawn_test_server() -> (TestService, TestSocket) {
        let (service, socket) = tower_lsp::LspService::new(LsharpBackend::new);
        let service = params_normalizer::ParamsNormalizer::new(service);
        (service, socket)
    }

    async fn initialize_test_server() -> (TestService, TestSocket) {
        let (mut service, socket) = spawn_test_server();
        let initialize_response = send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": null,
                    "capabilities": {}
                }
            }),
        )
        .await;
        assert!(
            initialize_response
                .as_ref()
                .is_some_and(|response| response.is_ok()),
            "initialize request は成功 response を返すべき"
        );

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            }),
        )
        .await;

        (service, socket)
    }

    async fn send_lsp_frame(service: &mut TestService, body: &Value) -> Option<Response> {
        let request: Request = serde_json::from_value(body.clone()).expect("request should parse");
        service.call(request).await.expect("request should succeed")
    }

    async fn read_lsp_message(socket: &mut TestSocket) -> Value {
        let message = tokio::time::timeout(std::time::Duration::from_secs(5), socket.next())
            .await
            .expect("timed out while reading lsp message")
            .expect("client socket should stay open");
        serde_json::to_value(message).expect("message should serialize")
    }

    async fn read_publish_diagnostics(socket: &mut TestSocket) -> Value {
        loop {
            let message = read_lsp_message(socket).await;
            if message["method"].as_str() == Some("textDocument/publishDiagnostics") {
                return message;
            }
        }
    }
}
