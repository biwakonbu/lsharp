mod analysis;
mod completion;
mod format;
mod params_normalizer;
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
    /// initialize で受け取った workspace root 群
    workspace_roots: Arc<RwLock<Vec<std::path::PathBuf>>>,
    /// Rust LSP 向け single-file incremental compile cache
    compilation_cache: Arc<RwLock<CompilationCache>>,
}

impl LsharpBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            source_cache: Arc::new(RwLock::new(HashMap::new())),
            version_cache: Arc::new(RwLock::new(HashMap::new())),
            workspace_roots: Arc::new(RwLock::new(Vec::new())),
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

    fn set_workspace_roots(&self, roots: Vec<std::path::PathBuf>) {
        if let Ok(mut workspace_roots) = self.workspace_roots.write() {
            *workspace_roots = roots;
        }
    }

    fn workspace_roots_snapshot(&self) -> Vec<std::path::PathBuf> {
        self.workspace_roots
            .read()
            .ok()
            .map(|roots| roots.clone())
            .unwrap_or_default()
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

    fn open_document_sources(&self) -> HashMap<Url, String> {
        self.source_cache
            .read()
            .ok()
            .map(|cache| cache.clone())
            .unwrap_or_default()
    }

    fn version_snapshot(&self) -> HashMap<Url, i32> {
        self.version_cache
            .read()
            .ok()
            .map(|versions| versions.clone())
            .unwrap_or_default()
    }

    fn affected_file_uris(changed_uri: &Url, documents: &HashMap<Url, String>) -> Vec<Url> {
        use std::collections::{HashMap, HashSet, VecDeque};

        let mut uri_by_module = HashMap::new();
        let mut module_by_uri = HashMap::new();
        let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();

        for (uri, source) in documents {
            if uri.scheme() != "file" {
                continue;
            }
            let Ok(path) = uri.to_file_path() else {
                continue;
            };
            let module_name = util::module_name_from_source(source, &path);
            let imports = util::imported_modules_from_source(source);

            uri_by_module.insert(module_name.clone(), uri.clone());
            module_by_uri.insert(uri.clone(), module_name.clone());
            for import in imports {
                reverse_deps
                    .entry(import)
                    .or_default()
                    .push(module_name.clone());
            }
        }

        let Some(changed_module) = module_by_uri.get(changed_uri).cloned() else {
            return vec![changed_uri.clone()];
        };

        let mut ordered = vec![changed_uri.clone()];
        let mut visited = HashSet::from([changed_module.clone()]);
        let mut queue = VecDeque::from([changed_module]);
        while let Some(module_name) = queue.pop_front() {
            let mut dependents = reverse_deps.remove(&module_name).unwrap_or_default();
            dependents.sort();
            for dependent in dependents {
                if !visited.insert(dependent.clone()) {
                    continue;
                }
                queue.push_back(dependent.clone());
                if let Some(uri) = uri_by_module.get(&dependent) {
                    ordered.push(uri.clone());
                }
            }
        }

        ordered
    }

    fn workspace_file_documents(
        workspace_roots: &[std::path::PathBuf],
        open_documents: &HashMap<Url, String>,
    ) -> HashMap<Url, String> {
        fn collect_lsharp_files(root: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
            let Ok(entries) = std::fs::read_dir(root) else {
                return;
            };
            let mut paths: Vec<_> = entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    collect_lsharp_files(&path, files);
                } else if path.extension().and_then(|ext| ext.to_str()) == Some("ls") {
                    files.push(path);
                }
            }
        }

        let mut documents = HashMap::new();
        for (uri, source) in open_documents {
            if uri.scheme() == "file" {
                documents.insert(uri.clone(), source.clone());
            }
        }

        let mut files = Vec::new();
        for root in workspace_roots {
            collect_lsharp_files(root, &mut files);
        }
        files.sort();
        files.dedup();

        for path in files {
            let Ok(uri) = Url::from_file_path(&path) else {
                continue;
            };
            if documents.contains_key(&uri) {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            documents.insert(uri, source);
        }

        documents
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
        let source_overrides = self.file_source_overrides();
        let open_documents = self.open_document_sources();
        let workspace_roots = self.workspace_roots_snapshot();
        let version_snapshot = self.version_snapshot();
        let workspace_documents = Self::workspace_file_documents(&workspace_roots, &open_documents);
        let affected_uris = Self::affected_file_uris(&uri, &workspace_documents);
        tokio::spawn(async move {
            let full_diagnostics = tokio::task::spawn_blocking(move || {
                let mut cache = compilation_cache
                    .write()
                    .expect("compilation cache lock should be available");
                affected_uris
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, affected_uri)| {
                        let source = workspace_documents.get(&affected_uri)?.clone();
                        let version = version_snapshot.get(&affected_uri).copied();
                        if index > 0
                            && affected_uri.scheme() == "file"
                            && let Ok(path) = affected_uri.to_file_path()
                        {
                            let module_name = util::module_name_from_source(&source, &path);
                            cache.remove_module(&module_name);
                        }
                        let diagnostics = util::parse_and_check_uri_incremental(
                            &affected_uri,
                            &source,
                            &source_overrides,
                            &mut cache,
                        );
                        Some((affected_uri, diagnostics, version))
                    })
                    .collect::<Vec<_>>()
            })
            .await
            .expect("full diagnostics task should complete");

            for (affected_uri, diagnostics, affected_version) in full_diagnostics {
                if affected_version.is_none()
                    || LsharpBackend::is_current_version(
                        &version_cache,
                        &affected_uri,
                        affected_version.expect("checked above"),
                    )
                {
                    client
                        .publish_diagnostics(affected_uri, diagnostics, affected_version)
                        .await;
                }
            }
        });
    }
}

fn text_document_sync_kind() -> TextDocumentSyncKind {
    TextDocumentSyncKind::INCREMENTAL
}

#[tower_lsp::async_trait]
impl LanguageServer for LsharpBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut workspace_roots = Vec::new();
        if let Some(folders) = params.workspace_folders {
            workspace_roots.extend(
                folders
                    .into_iter()
                    .filter_map(|folder| folder.uri.to_file_path().ok()),
            );
        } else if let Some(root_uri) = params.root_uri
            && let Ok(root_path) = root_uri.to_file_path()
        {
            workspace_roots.push(root_path);
        }
        self.set_workspace_roots(workspace_roots);

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
include!("lib_tests.rs");
