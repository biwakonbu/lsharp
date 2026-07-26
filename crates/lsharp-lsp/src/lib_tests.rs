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
        assert_eq!(range, Range::new(Position::new(0, 6), Position::new(0, 9)));
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

    #[test]
    fn test_protocol_surface_keeps_incremental_sync_and_formatting() {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(text_document_sync_kind())),
            document_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        };

        assert!(matches!(
            capabilities.text_document_sync,
            Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL
            ))
        ));
        assert_eq!(
            capabilities.document_formatting_provider,
            Some(OneOf::Left(true))
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

    #[tokio::test]
    async fn test_did_open_dependency_republishes_dependent_open_file_diagnostics() {
        let workspace = unique_temp_dir("lsharp_lsp_dependent_republish");
        std::fs::create_dir_all(&workspace).unwrap();
        let main_source = "(module Main)\n(import Helpers)\n(defn main [] (+ (helper) 1))\n";
        let helpers_clean = "(module Helpers)\n(defn helper [] 1)\n";
        let helpers_dirty = "(module Helpers)\n(defn helper [] true)\n";
        std::fs::write(workspace.join("Main.ls"), main_source).unwrap();
        std::fs::write(workspace.join("Helpers.ls"), helpers_clean).unwrap();

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
                        "uri": main_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": main_source,
                    }
                }
            }),
        )
        .await;
        let _ = read_publish_diagnostics(&mut socket).await;
        let main_full = read_publish_diagnostics_for_uri(&mut socket, main_uri.as_str()).await;
        assert_eq!(
            main_full["params"]["diagnostics"]
                .as_array()
                .map(std::vec::Vec::len),
            Some(0),
            "clean dependency の初期状態では Main diagnostics は空のままのはず"
        );

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
                        "text": helpers_dirty,
                    }
                }
            }),
        )
        .await;
        let _ = read_publish_diagnostics(&mut socket).await;
        let republished_main =
            read_publish_diagnostics_for_uri(&mut socket, main_uri.as_str()).await;
        assert!(
            republished_main["params"]["diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty()),
            "dependency を開いた結果、dependent open file の Main diagnostics も再 publish されるべき"
        );

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[tokio::test]
    async fn test_hover_returns_while_background_full_diagnostics_runs() {
        let (document_source, _, _, _) = benchmark_document_fixture();
        let busy_uri = "file:///busy-hover-test.ls";
        let hover_uri = "file:///hover-fast-test.ls";
        let hover_source = "(defn add [x y] (+ x y))\n(defn main [] (add 1 2))\n";
        let (mut service, mut socket) = initialize_test_server().await;

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": hover_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": hover_source,
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
                        "uri": busy_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": document_source,
                    }
                }
            }),
        )
        .await;

        let start = std::time::Instant::now();
        let hover_response = send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "id": 200,
                "method": "textDocument/hover",
                "params": {
                    "textDocument": { "uri": hover_uri },
                    "position": { "line": 1, "character": 16 }
                }
            }),
        )
        .await;
        let elapsed = start.elapsed();

        let response = hover_response.expect("hover request should return a response");
        let payload = response
            .result()
            .expect("hover response should be successful");
        assert!(
            payload["contents"].is_object() || payload["contents"].is_string(),
            "hover result should contain contents: {payload:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "background diagnostics 中でも hover は 50ms 未満で返るべき: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn test_completion_returns_while_background_full_diagnostics_runs() {
        let (document_source, _, _, _) = benchmark_document_fixture();
        let busy_uri = "file:///busy-completion-test.ls";
        let completion_uri = "file:///completion-fast-test.ls";
        let completion_source = "(defn helper [] 1)\n(defn main [] (hel))\n";
        let (mut service, mut socket) = initialize_test_server().await;

        send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": completion_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": completion_source,
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
                        "uri": busy_uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": document_source,
                    }
                }
            }),
        )
        .await;

        let start = std::time::Instant::now();
        let completion_response = send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "id": 201,
                "method": "textDocument/completion",
                "params": {
                    "textDocument": { "uri": completion_uri },
                    "position": { "line": 1, "character": 19 }
                }
            }),
        )
        .await;
        let elapsed = start.elapsed();

        let response = completion_response.expect("completion request should return a response");
        let payload = response
            .result()
            .expect("completion response should be successful");
        let items = payload
            .as_array()
            .expect("completion result should be an array response");
        assert!(
            items
                .iter()
                .any(|item| item["label"].as_str() == Some("helper")),
            "completion は helper 候補を返すべき: {items:?}"
        );
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "background diagnostics 中でも completion は 50ms 未満で返るべき: {elapsed:?}"
        );
    }

    #[tokio::test]
    async fn test_did_open_republishes_unopened_workspace_dependent_diagnostics() {
        let workspace = unique_temp_dir("lsharp_lsp_workspace_dependent_republish");
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

        let workspace_uri = Url::from_directory_path(&workspace)
            .expect("temp workspace path should convert to file url");
        let main_uri = Url::from_file_path(workspace.join("Main.ls"))
            .expect("temp main path should convert to file url");
        let helpers_uri = Url::from_file_path(workspace.join("Helpers.ls"))
            .expect("temp helpers path should convert to file url");
        let (mut service, mut socket) = initialize_test_server_with_root(Some(workspace_uri)).await;

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
        let main_diagnostics =
            read_publish_diagnostics_for_uri(&mut socket, main_uri.as_str()).await;
        assert!(
            main_diagnostics["params"]["diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty()),
            "workspace root があれば、未 open の dependent Main にも diagnostics を再 publish するべき"
        );

        let _ = std::fs::remove_dir_all(&workspace);
    }

    #[tokio::test]
    async fn test_did_open_skips_unrelated_workspace_diagnostics_publish() {
        let workspace = unique_temp_dir("lsharp_lsp_workspace_skip_unrelated");
        std::fs::create_dir_all(&workspace).unwrap();
        std::fs::write(
            workspace.join("Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] (+ (helper) 1))\n",
        )
        .unwrap();
        std::fs::write(
            workspace.join("Helpers.ls"),
            "(module Helpers)\n(defn helper [] true)\n",
        )
        .unwrap();
        std::fs::write(
            workspace.join("Unrelated.ls"),
            "(module Unrelated)\n(defn unrelated [] 42)\n",
        )
        .unwrap();

        let workspace_uri = Url::from_directory_path(&workspace)
            .expect("temp workspace path should convert to file url");
        let unrelated_uri = Url::from_file_path(workspace.join("Unrelated.ls"))
            .expect("temp unrelated path should convert to file url");
        let helpers_uri = Url::from_file_path(workspace.join("Helpers.ls"))
            .expect("temp helpers path should convert to file url");
        let (mut service, mut socket) = initialize_test_server_with_root(Some(workspace_uri)).await;

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
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(200),
                read_publish_diagnostics_for_uri(&mut socket, unrelated_uri.as_str())
            )
            .await
            .is_err(),
            "dirty-set 外の unrelated workspace file は diagnostics を再 publish しないべき"
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
        let target_line = "(defn helper-500 [] 500)".to_string();
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
        initialize_test_server_with_root(None).await
    }

    async fn initialize_test_server_with_root(root_uri: Option<Url>) -> (TestService, TestSocket) {
        let (mut service, socket) = spawn_test_server();
        let initialize_response = send_lsp_frame(
            &mut service,
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "processId": null,
                    "rootUri": root_uri,
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

    async fn read_publish_diagnostics_for_uri(socket: &mut TestSocket, uri: &str) -> Value {
        loop {
            let message = read_publish_diagnostics(socket).await;
            if message["params"]["uri"].as_str() == Some(uri) {
                return message;
            }
        }
    }
}
