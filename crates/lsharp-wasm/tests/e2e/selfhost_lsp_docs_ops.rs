use super::support::*;

fn selfhost_doctools_runtime_bundle() -> String {
    [
        include_str!("../../../../selfhost/Token.ls"),
        include_str!("../../../../selfhost/AST.ls"),
        include_str!("../../../../selfhost/Lexer.ls"),
        include_str!("../../../../selfhost/Parser.ls"),
        include_str!("../../../../selfhost/DocTools.ls"),
    ]
    .join("\n")
}

fn run_lsp_harness(_name: &str, harness: &str) -> Vec<String> {
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

fn lsp_diagnostic_helpers_source() -> String {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lsp_path = project_root.join("selfhost/LspServer.ls");
    let source = std::fs::read_to_string(&lsp_path)
        .unwrap_or_else(|e| panic!("{} の読み込みに失敗: {}", lsp_path.display(), e));
    let start = source
        .find(";; === 診断の安定順序制御")
        .expect("diagnostics section start が見つからない");
    let end = source
        .find(";; === 診断 JSON text renderer ===")
        .expect("diagnostics section end が見つからない");
    source[start..end].to_string()
}

fn run_lsp_diagnostic_harness(harness: &str) -> Vec<String> {
    let source = lsp_diagnostic_helpers_source();
    let output = compile_and_run(&format!("{}\n{}", source, harness));
    output
        .trim()
        .lines()
        .map(std::string::ToString::to_string)
        .collect()
}

/// TEST-LSP-03: selfhost/LspServer.ls に diagnostics の安定ソート機構
///
/// T4b-3 AC-208/AC-209/AC-210/AC-211: 診断のグルーピング・ソート・重複マージ・決定的順序
/// Red Phase: selfhost/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_diagnostic_ordering() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lsp_path = project_root.join("selfhost/LspServer.ls");
    assert!(lsp_path.exists(), "selfhost/LspServer.ls が存在しない");
    let source =
        std::fs::read_to_string(&lsp_path).expect("selfhost/LspServer.ls の読み込みに失敗");

    // T4b-3 AC-208: 診断は source フィールドでグルーピングされ行番号昇順
    assert!(
        source.contains("sort") || source.contains("order") || source.contains("diagnostic"),
        "selfhost/LspServer.ls に diagnostics のソート/順序制御がない (AC-208)"
    );
}

/// TEST-LSP-04: selfhost/LspServer.ls の主要ハンドラが runtime で観測できること
#[test]
fn test_e2e_selfhost_lsp_runtime_handlers() {
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[0], "7", "initialize capability count は 7 であるべき");
    assert_eq!(lines[1], "1", "textDocumentSync=Full であるべき");
    assert_eq!(lines[2], "1", "hoverProvider=true であるべき");
    assert_eq!(lines[3], "1", "completionProvider=true であるべき");
    assert_eq!(lines[4], "12", "didOpen は source length=12 を返すべき");
    assert_eq!(lines[5], "8", "didChange は source length=8 を返すべき");
    assert_eq!(lines[6], "1", "formatting は edit count=1 を返すべき");
    assert_eq!(lines[7], "7", "completion は keyword count=7 を返すべき");
    assert_eq!(lines[8], "0", "shutdown は 0 を返すべき");
}

/// TEST-LSP-05: selfhost/LspServer.ls の sort-diagnostics が 2 要素を行番号順に並べること
#[test]
fn test_e2e_selfhost_lsp_runtime_sort_diagnostics() {
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[9], "1010001",
        "先頭 diagnostic key は source=0,sev=1,line=1,col=1 であるべき"
    );
    assert_eq!(
        lines[10], "1030002",
        "次の diagnostic key は source=0,sev=1,line=3,col=2 であるべき"
    );
}

/// TEST-LSP-06: selfhost/LspServer.ls の merge-duplicate-diagnostics が同一 span を 1 件へ潰すこと
#[test]
fn test_e2e_selfhost_lsp_runtime_merge_duplicates() {
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[11], "1", "merged diagnostics count は 1 であるべき");
    assert_eq!(
        lines[12], "1",
        "merged diagnostics severity は高い方=1 を残すべき"
    );
}

/// TEST-LSP-07: selfhost/LspServer.ls の navigation handler shape が runtime で観測できること
#[test]
fn test_e2e_selfhost_lsp_runtime_navigation_shapes() {
    let source = selfhost_lsp_runtime_bundle();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[13], "2",
        "hover response shape length は 2 であるべき"
    );
    assert_eq!(
        lines[14], "3",
        "goto-definition shape length は 3 であるべき"
    );
    assert_eq!(lines[15], "1", "references count は 1 であるべき");
    assert_eq!(lines[16], "1", "rename changes length は 1 であるべき");
}

/// TEST-LSP-08: selfhost/LspServer.ls が JsonRpc method 定数で dispatch できること
#[test]
fn test_e2e_selfhost_lsp_runtime_jsonrpc_method_dispatch() {
    let source = selfhost_lsp_runtime_bundle();
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        init (json-rpc-dispatch (lsp-method-initialize) 0 state)
        did-open (json-rpc-dispatch (lsp-method-did-open) 14 state)
        did-change (json-rpc-dispatch (lsp-method-did-change) 9 state)
        hover (json-rpc-dispatch (lsp-method-hover) 0 state)
        goto-def (json-rpc-dispatch (lsp-method-goto-def) 0 state)
        formatting (json-rpc-dispatch (lsp-method-formatting) 0 state)
        completion (json-rpc-dispatch (lsp-method-completion) 0 state)
        shutdown (json-rpc-dispatch (lsp-method-shutdown) 0 state)]
    (do
      (print (vector-length init))
      (print did-open)
      (print did-change)
      (print (vector-length hover))
      (print (vector-length goto-def))
      (print (vector-length formatting))
      (print (vector-length completion))
      (print shutdown)
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0], "7",
        "initialize dispatch は capability vector を返すべき"
    );
    assert_eq!(
        lines[1], "14",
        "didOpen dispatch は source length を返すべき"
    );
    assert_eq!(
        lines[2], "9",
        "didChange dispatch は source length を返すべき"
    );
    assert_eq!(
        lines[3], "2",
        "hover dispatch は response shape length=2 を返すべき"
    );
    assert_eq!(
        lines[4], "3",
        "goto-definition dispatch は shape length=3 を返すべき"
    );
    assert_eq!(
        lines[5], "1",
        "formatting dispatch は edit count=1 を返すべき"
    );
    assert_eq!(
        lines[6], "7",
        "completion dispatch は keyword count=7 を返すべき"
    );
    assert_eq!(lines[7], "0", "shutdown dispatch は 0 を返すべき");
}

/// TEST-LSP-09: selfhost/LspServer.ls の server-loop が 1 メッセージ dispatch を観測できること
#[test]
fn test_e2e_selfhost_lsp_runtime_server_loop_single_message() {
    let source = selfhost_lsp_runtime_bundle();
    let harness = r#"
(defn make-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn make-doc-params [uri src]
  (let [v (vector-new 2)]
    (vector-push (vector-push v uri) src)))

(defn main []
  (let [open-req (make-loop-request (lsp-method-did-open) (make-doc-params 1 "abcdefghijklmno"))
        change-req (make-loop-request (lsp-method-did-change) (make-doc-params 1 "abcdefghi"))
        completion-req (make-loop-request (lsp-method-completion) 0)]
    (do
      (print (server-loop open-req))
      (print (server-loop change-req))
      (print (vector-length (server-loop completion-req)))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0], "15",
        "server-loop は didOpen request を dispatch できるべき"
    );
    assert_eq!(
        lines[1], "9",
        "server-loop は didChange request を dispatch できるべき"
    );
    assert_eq!(
        lines[2], "7",
        "server-loop は completion request を dispatch できるべき"
    );
}

/// TEST-LSP-09b: server-loop-step が shared state 上で複数 request を順に dispatch できること
#[test]
fn test_e2e_selfhost_lsp_runtime_server_loop_stateful_sequence() {
    let source = selfhost_lsp_runtime_bundle();
    let open_src_literal = "(defn helper [] 1)\\n(defn main [] (helper 1))";
    let change_src_literal = "(defn helper [] 1)\\n(defn main []  (he))";

    let harness = format!(
        r#"
(defn make-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn make-doc-params [uri src]
  (let [v (vector-new 2)]
    (vector-push (vector-push v uri) src)))

(defn main []
  (let [state (server-state-new)
        init-req (make-loop-request (lsp-method-initialize) 0)
        open-req (make-loop-request (lsp-method-did-open) (make-doc-params 77 "{open_src_literal}"))
        change-req (make-loop-request (lsp-method-did-change) (make-doc-params 77 "{change_src_literal}"))
        completion-params (vector-push (vector-push (vector-push (vector-new 3) 77) 2) 19)
        completion-req (make-loop-request (lsp-method-completion) completion-params)
        init (server-loop-step state init-req)
        open (server-loop-step state open-req)
        change (server-loop-step state change-req)
        completion (server-loop-step state completion-req)]
    (do
      (print (vector-length init))
      (print open)
      (print change)
      (print (vector-length completion))
      (print (server-state-doc-count state))
      (print (server-state-request-count state))
      (print (server-state-source-length state))
      0)))
"#
    );

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0], "7",
        "server-loop-step initialize は 7 slot capability vector を返すべき"
    );
    assert_eq!(
        lines[1],
        "(defn helper [] 1)\n(defn main [] (helper 1))"
            .len()
            .to_string(),
        "server-loop-step didOpen は source length を返すべき"
    );
    assert_eq!(
        lines[2],
        "(defn helper [] 1)\n(defn main []  (he))".len().to_string(),
        "server-loop-step didChange は更新後 source length を返すべき"
    );
    assert_eq!(
        lines[3], "1",
        "server-loop-step completion は session source から 1 件返すべき"
    );
    assert_eq!(
        lines[4], "1",
        "shared state の open document 数は 1 のままであるべき"
    );
    assert_eq!(
        lines[5], "4",
        "shared state の request count は 4 件まで蓄積されるべき"
    );
    assert_eq!(
        lines[6],
        "(defn helper [] 1)\n(defn main []  (he))".len().to_string(),
        "shared state の source length は didChange 後の最新値を保持するべき"
    );
}

/// TEST-LSP-09c: initialize / shutdown が server-state lifecycle flag を更新すること
#[test]
fn test_e2e_selfhost_lsp_runtime_lifecycle_flags() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn make-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn main []
  (let [state (server-state-new)
        _ (server-loop-step state (make-loop-request (lsp-method-initialize) 0))
        _ (server-loop-step state (make-loop-request (lsp-method-shutdown) 0))]
    (do
      (print (server-state-initialized state))
      (print (server-state-shutdown state))
      (print (server-state-request-count state))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[0], "1", "initialize 後は initialized flag が立つべき");
    assert_eq!(lines[1], "1", "shutdown 後は shutdown flag が立つべき");
    assert_eq!(
        lines[2], "2",
        "initialize + shutdown で request count は 2 になるべき"
    );
}

/// TEST-LSP-09d: server-loop-sequence が shared state で request 群を順に処理できること
#[test]
fn test_e2e_selfhost_lsp_runtime_server_loop_sequence() {
    let source = selfhost_lsp_runtime_bundle();
    let open_src_literal = "(defn helper [] 1)\\n(defn main [] (helper 1))";
    let change_src_literal = "(defn helper [] 1)\\n(defn main []  (he))";

    let harness = format!(
        r#"
(defn make-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn make-doc-params [uri src]
  (let [v (vector-new 2)]
    (vector-push (vector-push v uri) src)))

(defn main []
  (let [completion-params (vector-push (vector-push (vector-push (vector-new 3) 77) 2) 19)
        requests (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 4)
                         (make-loop-request (lsp-method-initialize) 0))
                       (make-loop-request (lsp-method-did-open) (make-doc-params 77 "{open_src_literal}")))
                     (make-loop-request (lsp-method-did-change) (make-doc-params 77 "{change_src_literal}")))
                   (make-loop-request (lsp-method-completion) completion-params))
        summary (server-loop-sequence requests)
        results (vector-get summary 0)]
    (do
      (print (vector-length results))
      (print (vector-length (vector-get results 0)))
      (print (vector-get results 1))
      (print (vector-get results 2))
      (print (vector-length (vector-get results 3)))
      (print (vector-get summary 1))
      (print (vector-get summary 2))
      (print (vector-get summary 3))
      0)))
"#
    );

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0], "4",
        "server-loop-sequence は 4 件の結果を返すべき"
    );
    assert_eq!(
        lines[1], "7",
        "1 件目は initialize capability vector であるべき"
    );
    assert_eq!(
        lines[2],
        "(defn helper [] 1)\n(defn main [] (helper 1))"
            .len()
            .to_string(),
        "2 件目は didOpen source length であるべき"
    );
    assert_eq!(
        lines[3],
        "(defn helper [] 1)\n(defn main []  (he))".len().to_string(),
        "3 件目は didChange source length であるべき"
    );
    assert_eq!(lines[4], "1", "4 件目は completion 1 件であるべき");
    assert_eq!(lines[5], "1", "summary の document count は 1 件であるべき");
    assert_eq!(lines[6], "4", "summary の request count は 4 件であるべき");
    assert_eq!(
        lines[7],
        "(defn helper [] 1)\n(defn main []  (he))".len().to_string(),
        "summary の source length は最新 document に一致するべき"
    );
}

/// TEST-LSP-08: sort-diagnostics が 3 要素以上を行番号順にソートできること
#[test]
fn test_e2e_selfhost_lsp_sort_diagnostics_three() {
    let source = selfhost_lsp_runtime_bundle();

    // 3 つの診断を逆順で作成し、sort-diagnostics でソートされることを検証
    let harness = r#"
(defn make-diag [sev rule line col msg src]
  (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) sev) rule) line) col) msg) src))

(defn main []
  (let [d1 (make-diag 1 100 5 1 0 0)
        d2 (make-diag 1 101 1 3 0 0)
        d3 (make-diag 1 102 3 2 0 0)
        diags (vector-push (vector-push (vector-push (vector-new 3) d1) d2) d3)
        sorted (sort-diagnostics diags)]
    (do
      (print (diagnostic-order-key (vector-get sorted 0)))
      (print (diagnostic-order-key (vector-get sorted 1)))
      (print (diagnostic-order-key (vector-get sorted 2)))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "sort 3 diagnostics 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1010003",
        "最小の diagnostic key (source=0,sev=1,line=1,col=3) が先頭"
    );
    assert_eq!(
        lines[1], "1030002",
        "中間の diagnostic key (source=0,sev=1,line=3,col=2) が 2 番目"
    );
    assert_eq!(
        lines[2], "1050001",
        "最大の diagnostic key (source=0,sev=1,line=5,col=1) が末尾"
    );
}

/// TEST-LSP-09e: shared state が複数 URI の document source を保持できること
#[test]
fn test_e2e_selfhost_lsp_runtime_multi_document_state() {
    let source = selfhost_lsp_runtime_bundle();
    let src_a = "(defn alpha [] 1)";
    let src_b = "(defn beta [] 2)";
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 10 "{src_a}")
        _ (server-state-open-document state 20 "{src_b}")]
    (do
      (print (server-state-doc-count state))
      (print (string-length (server-state-source-for-uri state 10)))
      (print (string-length (server-state-source-for-uri state 20)))
      (print (server-state-source-length state))
      0)))
"#
    );
    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines[0], "2",
        "異なる URI の didOpen は doc-count=2 に増えるべき"
    );
    assert_eq!(
        lines[1],
        src_a.len().to_string(),
        "uri=10 の source を保持するべき"
    );
    assert_eq!(
        lines[2],
        src_b.len().to_string(),
        "uri=20 の source を保持するべき"
    );
    assert_eq!(
        lines[3],
        src_b.len().to_string(),
        "current source は最後に開いた document を保持するべき"
    );
}

/// TEST-LSP-10: handle-hover が型情報文字列を返すこと
#[test]
fn test_e2e_selfhost_lsp_hover_returns_type_info() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [state (server-state-new)
        ;; params: [uri, line, col]
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 10) 5)
        result (handle-hover params state)]
    (do
      ;; result は [range, contents] の 2 要素
      (print (vector-length result))
      ;; contents スロットに型情報文字列が格納されている
      (print-string (vector-get result 1))
      (print-string "\n")
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "hover 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "2", "hover response は 2 要素であるべき");
    assert_eq!(
        lines[1], "type-info:10:5",
        "hover contents は型情報 text であるべき"
    );
}

/// TEST-LSP-11: handle-goto-definition がソース位置構造を返すこと
#[test]
fn test_e2e_selfhost_lsp_definition_returns_location() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [state (server-state-new)
        ;; params: [uri, line, col]
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 10) 5)
        result (handle-goto-definition params state)]
    (do
      ;; result は [uri, line, col] の 3 要素
      (print (vector-length result))
      (print (vector-get result 0))
      (print (vector-get result 1))
      (print (vector-get result 2))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 4, "definition 出力が不足: {:?}", lines);
    assert_eq!(
        lines[0], "3",
        "definition response は [uri, line, col] の 3 要素であるべき"
    );
}

/// TEST-LSP-12: handle-references が位置リストを返すこと
#[test]
fn test_e2e_selfhost_lsp_references_returns_locations() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [state (server-state-new)
        ;; params: [uri, line, col]
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 10) 5)
        result (handle-references params state)]
    (do
      ;; result は locations リスト (最低 1 要素)
      (print (vector-length result))
      ;; 各 location は [uri, line, col] の 3 要素
      (let [loc0 (vector-get result 0)]
        (do
          (print (vector-length loc0))
          0)))))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "references 出力が不足: {:?}", lines);
    let ref_count: i64 = lines[0].parse().unwrap_or(0);
    assert!(
        ref_count >= 1,
        "references は 1 件以上返すべき (got {})",
        ref_count
    );
    assert_eq!(
        lines[1], "3",
        "各 location は [uri, line, col] の 3 要素であるべき"
    );
}

/// TEST-LSP-13: handle-completion がキーワード補完候補を返すこと
#[test]
fn test_e2e_selfhost_lsp_completion_returns_keywords() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [state (server-state-new)
        params 0
        result (handle-completion params state)]
    (do
      ;; result は completion items のリスト
      (print (vector-length result))
      ;; 各 item は [label, kind, insertText] の 3 要素
      (let [item0 (vector-get result 0)]
        (do
          (print (vector-length item0))
          (print-string (vector-get item0 0))
          (print-string "\n")
          ;; kind=14 は Keyword
          (print (vector-get item0 1))
          (print-string (vector-get item0 2))
          (print-string "\n")
          0)))))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 5, "completion 出力が不足: {:?}", lines);
    let item_count: i64 = lines[0].parse().unwrap_or(0);
    assert!(
        item_count >= 7,
        "completion は 7 件以上のキーワードを返すべき (got {})",
        item_count
    );
    assert_eq!(
        lines[1], "3",
        "各 completion item は [label, kind, insertText] の 3 要素であるべき"
    );
    assert_eq!(lines[2], "defn", "先頭 keyword label は defn であるべき");
    assert_eq!(lines[3], "14", "completion kind は 14 (Keyword) であるべき");
    assert_eq!(
        lines[4], "defn",
        "先頭 keyword insertText は defn であるべき"
    );
}

/// TEST-LSP-14: sort-diagnostics が source 優先 → severity → line → col の順で並べること
#[test]
fn test_e2e_selfhost_lsp_diagnostic_ordering_source_priority() {
    let source = selfhost_lsp_runtime_bundle();

    // source=3(lint) が source=1(parse) より後に来ることを検証
    let harness = r#"
(defn make-diag [sev rule line col msg src]
  (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) sev) rule) line) col) msg) src))

(defn main []
  (let [;; source=3(lint), sev=1, line=1, col=1
        d1 (make-diag 1 200 1 1 0 3)
        ;; source=1(parse), sev=1, line=1, col=1
        d2 (make-diag 1 100 1 1 0 1)
        ;; source=2(type), sev=2, line=1, col=1
        d3 (make-diag 2 150 1 1 0 2)
        diags (vector-push (vector-push (vector-push (vector-new 3) d1) d2) d3)
        sorted (sort-diagnostics diags)]
    (do
      ;; source 順: parse(1) → type(2) → lint(3)
      (print (vector-get (vector-get sorted 0) 5))
      (print (vector-get (vector-get sorted 1) 5))
      (print (vector-get (vector-get sorted 2) 5))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "source priority 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "最初は source=1 (parse) であるべき");
    assert_eq!(lines[1], "2", "次は source=2 (type) であるべき");
    assert_eq!(lines[2], "3", "最後は source=3 (lint) であるべき");
}

/// TEST-LSP-15: dedup-diagnostics が同一 span で severity の高い方を残すこと
#[test]
fn test_e2e_selfhost_lsp_diagnostic_dedup() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn make-diag [sev rule line col msg src]
  (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) sev) rule) line) col) msg) src))

(defn main []
  (let [;; 同一 span (line=3, col=5) に severity 違い 3 つ
        d1 (make-diag 3 100 3 5 0 1)
        d2 (make-diag 1 101 3 5 0 1)
        d3 (make-diag 2 102 3 5 0 1)
        ;; 別 span (line=7, col=2) に 1 つ
        d4 (make-diag 2 103 7 2 0 1)
        diags (vector-push (vector-push (vector-push (vector-push (vector-new 4) d1) d2) d3) d4)
        deduped (dedup-diagnostics diags)]
    (do
      ;; 同一 span は 1 つに集約、別 span は残る → 2 件
      (print (vector-length deduped))
      ;; 同一 span は severity=1 (最高) が残る
      (print (vector-get (vector-get deduped 0) 0))
      ;; 別 span は severity=2 のまま
      (print (vector-get (vector-get deduped 1) 0))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "dedup 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "2", "dedup 後は 2 件であるべき");
    assert_eq!(lines[1], "1", "同一 span は severity=1 (最高) が残るべき");
    assert_eq!(lines[2], "2", "別 span は severity=2 のまま残るべき");
}

/// TEST-LSP-15a: 同一 source/severity/span は rule/message 順で安定ソートされること
#[test]
fn test_e2e_selfhost_lsp_diagnostic_sort_same_span_tiebreak() {
    let harness = r#"
(defn make-diag [sev rule line col msg src]
  (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) sev) rule) line) col) msg) src))

(defn main []
  (let [diag-b (make-diag 1 200 4 8 9001 1)
        diag-a (make-diag 1 100 4 8 7001 1)
        diag-c (make-diag 1 100 4 8 8001 1)
        diags (vector-push (vector-push (vector-push (vector-new 3) diag-b) diag-c) diag-a)
        sorted (sort-diagnostics diags)]
    (do
      (print (vector-get (vector-get sorted 0) 1))
      (print (vector-get (vector-get sorted 0) 4))
      (print (vector-get (vector-get sorted 1) 1))
      (print (vector-get (vector-get sorted 1) 4))
      (print (vector-get (vector-get sorted 2) 1))
      0)))
"#;

    let lines = run_lsp_diagnostic_harness(harness);

    assert!(lines.len() >= 5, "same-span sort 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "100", "rule が小さい diagnostic が先頭であるべき");
    assert_eq!(
        lines[1], "7001",
        "rule 同値なら messageHash が小さい方が先頭であるべき"
    );
    assert_eq!(lines[2], "100", "同一 rule の次要素も rule=100 であるべき");
    assert_eq!(lines[3], "8001", "messageHash が大きい方は後ろに来るべき");
    assert_eq!(lines[4], "200", "rule が大きい diagnostic は末尾であるべき");
}

/// TEST-LSP-15a: 同一 span / 同一 severity の parse/type 重複は parse を優先すること
#[test]
fn test_e2e_selfhost_lsp_diagnostic_dedup_prefers_parse_same_span() {
    let harness = r#"
(defn make-diag [sev rule line col msg src]
  (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) sev) rule) line) col) msg) src))

(defn main []
  (let [parse-diag (make-diag 1 100 4 8 7001 1)
        type-diag (make-diag 1 200 4 8 8001 2)
        lint-diag (make-diag 2 300 9 1 9001 3)
        diags (vector-push (vector-push (vector-push (vector-new 3) type-diag) lint-diag) parse-diag)
        sorted (sort-diagnostics diags)
        deduped (dedup-diagnostics sorted)]
    (do
      (print (vector-length deduped))
      (print (vector-get (vector-get deduped 0) 5))
      (print (vector-get (vector-get deduped 0) 1))
      (print (vector-get (vector-get deduped 1) 5))
      0)))
"#;

    let lines = run_lsp_diagnostic_harness(harness);

    assert!(lines.len() >= 4, "same-span dedup 出力が不足: {:?}", lines);
    assert_eq!(
        lines[0], "2",
        "同一 span の parse/type は 1 件へ集約されるべき"
    );
    assert_eq!(
        lines[1], "1",
        "同一 severity では parse(source=1) を優先するべき"
    );
    assert_eq!(lines[2], "100", "parse diagnostic の rule を保持するべき");
    assert_eq!(lines[3], "3", "別 span の lint diagnostic は残るべき");
}

/// TEST-LSP-15b: diagnostics を deterministic JSON text に render できること
#[test]
fn test_e2e_selfhost_lsp_render_diagnostic_json() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [diag (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 6) 2)
                       101)
                     3)
                   7)
                 9001)
               1)]
    (do
      (print-string (render-diagnostic-json diag))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    assert_eq!(
        output.trim(),
        r#"{"source":1,"severity":2,"rule":101,"line":3,"col":7,"messageHash":9001}"#,
        "render-diagnostic-json は固定順の JSON text を返すべき"
    );
}

/// TEST-LSP-15c: sort/dedup 後 diagnostics 群を deterministic JSON array に render できること
#[test]
fn test_e2e_selfhost_lsp_render_sorted_deduped_diagnostics_json() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [diag-a (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 6) 2)
                         201)
                       5)
                     9)
                   7001)
                 3)
        diag-b (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 6) 1)
                         202)
                       5)
                     9)
                   7002)
                 3)
        diag-c (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 6) 1)
                         203)
                       2)
                     4)
                   7003)
                 1)
        diags (vector-push (vector-push (vector-push (vector-new 3) diag-a) diag-b) diag-c)
        sorted (sort-diagnostics diags)
        deduped (dedup-diagnostics sorted)]
    (do
      (print-string (render-diagnostics-json deduped))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    assert_eq!(
        output.trim(),
        r#"[{"source":1,"severity":1,"rule":203,"line":2,"col":4,"messageHash":7003},{"source":3,"severity":1,"rule":202,"line":5,"col":9,"messageHash":7002}]"#,
        "render-diagnostics-json は sort/dedup 後の diagnostics を固定 JSON array で返すべき"
    );
}

fn lsp_diagnostic_snapshot(name: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/snapshots/lsp/diagnostics")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("snapshot 読み込み失敗 {}: {}", path.display(), e))
}

/// TEST-LSP-15d: single diagnostic JSON が snapshot file と一致すること
#[test]
fn test_e2e_selfhost_lsp_render_diagnostic_json_snapshot() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [diag (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 6) 2)
                       101)
                     3)
                   7)
                 9001)
               1)]
    (do
      (print-string (render-diagnostic-json diag))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let expected = lsp_diagnostic_snapshot("single-diagnostic.json");
    assert_eq!(
        output.trim(),
        expected.trim(),
        "single diagnostic JSON snapshot が一致するべき"
    );
}

/// TEST-LSP-15e: sort/dedup 後 diagnostics JSON array が snapshot file と一致すること
#[test]
fn test_e2e_selfhost_lsp_render_sorted_deduped_diagnostics_json_snapshot() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [diag-a (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 6) 2)
                         201)
                       5)
                     9)
                   7001)
                 3)
        diag-b (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 6) 1)
                         202)
                       5)
                     9)
                   7002)
                 3)
        diag-c (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push (vector-new 6) 1)
                         203)
                       2)
                     4)
                   7003)
                 1)
        diags (vector-push (vector-push (vector-push (vector-new 3) diag-a) diag-b) diag-c)
        sorted (sort-diagnostics diags)
        deduped (dedup-diagnostics sorted)]
    (do
      (print-string (render-diagnostics-json deduped))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let expected = lsp_diagnostic_snapshot("sorted-deduped-diagnostics.json");
    assert_eq!(
        output.trim(),
        expected.trim(),
        "sorted/dedup diagnostics JSON snapshot が一致するべき"
    );
}

/// TEST-LSP-16: encode-json-rpc-response が決定的な JSON-RPC 構造を生成すること
#[test]
fn test_e2e_selfhost_lsp_json_rpc_encode() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [response (encode-json-rpc-response 42 99)]
    (do
      ;; response は [jsonrpc-version, id, result] の 3 要素
      (print (vector-length response))
      ;; jsonrpc-version = 2 (JSON-RPC 2.0 を数値で表現)
      (print (vector-get response 0))
      ;; id = 42
      (print (vector-get response 1))
      ;; result = 99
      (print (vector-get response 2))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 4, "json-rpc encode 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "3", "JSON-RPC response は 3 要素であるべき");
    assert_eq!(lines[1], "2", "jsonrpc version は 2 であるべき");
    assert_eq!(lines[2], "42", "id は 42 であるべき");
    assert_eq!(lines[3], "99", "result は 99 であるべき");
}

/// TEST-LSP-17: parse-json-rpc-request が method + params を抽出すること
#[test]
fn test_e2e_selfhost_lsp_json_rpc_parse() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (let [;; msg = [jsonrpc-version, id, method-id, params]
        msg (vector-push (vector-push (vector-push (vector-push (vector-new 4) 2) 7) 21) 55)
        parsed (parse-json-rpc-request msg)]
    (do
      ;; parsed は [method-id, params] の 2 要素
      (print (vector-length parsed))
      ;; method-id = 21 (hover)
      (print (vector-get parsed 0))
      ;; params = 55
      (print (vector-get parsed 1))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "json-rpc parse 出力が不足: {:?}", lines);
    assert_eq!(
        lines[0], "2",
        "parsed request は [method-id, params] の 2 要素であるべき"
    );
    assert_eq!(lines[1], "21", "method-id は 21 (hover) であるべき");
    assert_eq!(lines[2], "55", "params は 55 であるべき");
}

/// TEST-LSP-17b: JSON-RPC error response が deterministic な wire shape を返すこと
#[test]
fn test_e2e_selfhost_lsp_json_rpc_error_encode() {
    let source = selfhost_lsp_runtime_bundle();

    let harness = r#"
(defn main []
  (do
    (print-string (render-json-rpc-error-response 42 -32601 "Method not found"))
    0))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);

    assert_eq!(
        output.trim(),
        r#"{"jsonrpc":"2.0","id":42,"error":{"code":-32601,"message":"Method not found"}}"#,
        "JSON-RPC error response は固定 shape の JSON text を返すべき"
    );
}

/// TEST-LSP-18: hover がソースとカーソル位置から実シンボル情報を返すこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_hover_uses_source_symbol() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src "(defn square [x] x)\n(defn main [] (square 1) (square 2))"
        params (vector-push (vector-push (vector-push (vector-push (vector-new 4) 99) 2) 17) src)
        result (handle-hover params state)
        range (vector-get result 0)]
    (do
      (print (vector-length result))
      (print (vector-get range 0))
      (print (vector-get range 1))
      (print (vector-get range 2))
      (print (vector-get range 3))
      (print-string (vector-get result 1))
      (print-string "\n")
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_hover", harness);

    assert_eq!(
        lines[0], "2",
        "hover response は [range, contents] の 2 要素であるべき"
    );
    assert_eq!(
        lines[1], "2",
        "hover range start-line は参照箇所の 2 行目であるべき"
    );
    assert_eq!(
        lines[2], "16",
        "hover range start-col は square 呼び出し先頭であるべき"
    );
    assert_eq!(
        lines[3], "2",
        "hover range end-line は参照箇所の 2 行目であるべき"
    );
    assert_eq!(
        lines[4], "22",
        "hover range end-col は symbol 終端であるべき"
    );
    assert_eq!(
        lines[5], "defn square",
        "hover contents は \"defn square\" text であるべき"
    );
}

/// TEST-LSP-19: definition / references がソース走査結果を返すこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_definition_and_references_use_source() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src "(defn square [x] x)\n(defn main [] (square 1) (square 2))"
        params (vector-push (vector-push (vector-push (vector-push (vector-new 4) 99) 2) 17) src)
        defn-loc (handle-goto-definition params state)
        refs (handle-references params state)
        ref0 (vector-get refs 0)
        ref1 (vector-get refs 1)
        ref2 (vector-get refs 2)]
    (do
      (print (vector-get defn-loc 0))
      (print (vector-get defn-loc 1))
      (print (vector-get defn-loc 2))
      (print (vector-length refs))
      (print (vector-get ref0 1))
      (print (vector-get ref0 2))
      (print (vector-get ref1 1))
      (print (vector-get ref1 2))
      (print (vector-get ref2 1))
      (print (vector-get ref2 2))
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_definition_references", harness);

    assert_eq!(lines[0], "99", "definition は元の uri を保持するべき");
    assert_eq!(
        lines[1], "1",
        "definition line は defn 名の 1 行目であるべき"
    );
    assert_eq!(
        lines[2], "7",
        "definition col は square 定義名の先頭であるべき"
    );
    assert_eq!(
        lines[3], "3",
        "references は定義 + 2 参照の合計 3 件であるべき"
    );
    assert_eq!(lines[4], "1", "1 件目は定義行であるべき");
    assert_eq!(lines[5], "7", "1 件目は定義名の列を指すべき");
    assert_eq!(lines[6], "2", "2 件目は 2 行目の呼び出しであるべき");
    assert_eq!(
        lines[7], "16",
        "2 件目は 1 つ目の square 呼び出しであるべき"
    );
    assert_eq!(lines[8], "2", "3 件目は 2 行目の呼び出しであるべき");
    assert_eq!(
        lines[9], "27",
        "3 件目は 2 つ目の square 呼び出しであるべき"
    );
}

/// TEST-LSP-19b: repeated symbol source では definition が直近の defn を安定して使うこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_definition_prefers_nearest_defn() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src (string-concat
              "(defn square [x] x)\n"
              (string-concat
                "(defn caller [] (square 1))\n"
                (string-concat
                  "(defn square [y] y)\n"
                  "(defn main [] (square 2) (square 3))")))
        params (vector-push (vector-push (vector-push (vector-push (vector-new 4) 99) 4) 17) src)
        defn-loc (handle-goto-definition params state)]
    (do
      (print (vector-get defn-loc 0))
      (print (vector-get defn-loc 1))
      (print (vector-get defn-loc 2))
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_definition_nearest_defn", harness);

    assert_eq!(lines[0], "99", "definition は元の uri を保持するべき");
    assert_eq!(
        lines[1], "3",
        "definition は直近の square defn 行を指すべき"
    );
    assert_eq!(
        lines[2], "7",
        "definition col は直近 defn 名の先頭であるべき"
    );
}

/// TEST-LSP-19d: source param なしでも uri に対応する open document から definition を解決できること
#[test]
fn test_e2e_selfhost_lsp_real_shapes_definition_uses_uri_document() {
    let src_a = "(defn alpha [] 1)\n(defn main [] (alpha))";
    let src_b = "(defn beta [] 2)\n(defn main [] (beta))";
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 10 "{src_a}")
        _ (server-state-open-document state 20 "{src_b}")
        params (vector-push (vector-push (vector-push (vector-new 3) 10) 2) 16)
        defn-loc (handle-goto-definition params state)]
    (do
      (print (vector-get defn-loc 0))
      (print (vector-get defn-loc 1))
      (print (vector-get defn-loc 2))
      0)))
"#
    );

    let lines = run_lsp_harness("lsp_real_shapes_definition_uses_uri_document", &harness);

    assert_eq!(lines[0], "10", "definition は要求 URI を返すべき");
    assert_eq!(lines[1], "1", "definition は uri=10 の 1 行目を指すべき");
    assert_eq!(
        lines[2], "7",
        "definition は alpha defn 名の先頭列を指すべき"
    );
}

/// TEST-LSP-19e: source param なしでも uri に対応する open document から hover contents を解決できること
#[test]
fn test_e2e_selfhost_lsp_real_shapes_hover_uses_uri_document() {
    let src_a = "(defn alphabet [x] x)\n(defn main [] (alphabet 1))";
    let src_b = "(defn b [x] x)\n(defn main [] (b 1))";
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 10 "{src_a}")
        _ (server-state-open-document state 20 "{src_b}")
        params (vector-push (vector-push (vector-push (vector-new 3) 10) 2) 19)
        result (handle-hover params state)]
    (do
      (print-string (vector-get result 1))
      0)))
"#
    );

    let lines = run_lsp_harness("lsp_real_shapes_hover_uses_uri_document", &harness);

    assert_eq!(
        lines[0], "defn alphabet",
        "hover は uri=10 の document symbol を返すべき"
    );
}

/// TEST-LSP-19f: open 済み別 document の defn へ cross-document definition を返せること
#[test]
fn test_e2e_selfhost_lsp_real_shapes_definition_resolves_open_document() {
    let helper_src = "(defn helper [x] x)";
    let main_src = "(import Helper)\n(defn main [] (helper 1))";
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 11 "{helper_src}")
        _ (server-state-open-document state 10 "{main_src}")
        params (vector-push (vector-push (vector-push (vector-new 3) 10) 2) 16)
        defn-loc (handle-goto-definition params state)]
    (do
      (print (vector-get defn-loc 0))
      (print (vector-get defn-loc 1))
      (print (vector-get defn-loc 2))
      0)))
"#
    );

    let lines = run_lsp_harness(
        "lsp_real_shapes_definition_resolves_open_document",
        &harness,
    );

    assert_eq!(
        lines[0], "11",
        "definition は helper document の uri を返すべき"
    );
    assert_eq!(
        lines[1], "1",
        "definition は helper defn の 1 行目を指すべき"
    );
    assert_eq!(
        lines[2], "7",
        "definition は helper defn 名の先頭列を指すべき"
    );
}

/// TEST-LSP-19g: open 済み別 document の defn へ cross-document hover contents を返せること
#[test]
fn test_e2e_selfhost_lsp_real_shapes_hover_resolves_open_document() {
    let helper_src = "(defn helper [x] x)";
    let main_src = "(import Helper)\n(defn main [] (helper 1))";
    let harness = format!(
        r#"
(defn main []
  (let [state (server-state-new)
        _ (server-state-open-document state 11 "{helper_src}")
        _ (server-state-open-document state 10 "{main_src}")
        params (vector-push (vector-push (vector-push (vector-new 3) 10) 2) 16)
        hover (handle-hover params state)]
    (do
      (print-string (vector-get hover 1))
      0)))
"#
    );

    let lines = run_lsp_harness("lsp_real_shapes_hover_resolves_open_document", &harness);

    assert_eq!(
        lines[0], "defn helper",
        "hover は helper document の defn contents を返すべき"
    );
}

/// TEST-LSP-19c: repeated symbol source でも hover は選択した occurrence の range を保つこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_hover_keeps_selected_occurrence_with_repeated_defns() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src (string-concat
              "(defn square [x] x)\n"
              (string-concat
                "(defn caller [] (square 1))\n"
                (string-concat
                  "(defn square [y] y)\n"
                  "(defn main [] (square 2) (square 3))")))
        params (vector-push (vector-push (vector-push (vector-push (vector-new 4) 99) 4) 17) src)
        result (handle-hover params state)
        range (vector-get result 0)]
    (do
      (print (vector-length result))
      (print (vector-get range 0))
      (print (vector-get range 1))
      (print (vector-get range 2))
      (print (vector-get range 3))
      (print-string (vector-get result 1))
      (print-string "\n")
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_hover_repeated_defns", harness);

    assert_eq!(
        lines[0], "2",
        "hover response は [range, contents] の 2 要素であるべき"
    );
    assert_eq!(
        lines[1], "4",
        "hover range start-line は 4 行目の呼び出しであるべき"
    );
    assert_eq!(
        lines[2], "16",
        "hover range start-col は 4 行目の最初の square 呼び出しであるべき"
    );
    assert_eq!(
        lines[3], "4",
        "hover range end-line は 4 行目の呼び出しであるべき"
    );
    assert_eq!(
        lines[4], "22",
        "hover range end-col は square 終端であるべき"
    );
    assert_eq!(
        lines[5], "defn square",
        "hover contents は安定して defn text を返すべき"
    );
}

/// TEST-LSP-20: completion が prefix とシンボル表から安定した item を返すこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_rename_returns_workspace_edit() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src "(defn square [x] x)\n(defn main [] (square 1) (square 2))"
        params (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push (vector-new 5) 99)
                       2)
                     17)
                   src)
                 "cube")
        changes (handle-rename params state)
        change0 (vector-get changes 0)
        edits (vector-get change0 1)
        edit0 (vector-get edits 0)
        edit1 (vector-get edits 1)
        edit2 (vector-get edits 2)]
    (do
      (print (vector-length changes))
      (print (vector-length change0))
      (print (vector-get change0 0))
      (print (vector-length edits))
      (print (vector-get edit0 0))
      (print (vector-get edit0 1))
      (print (vector-get edit0 2))
      (print (vector-get edit0 3))
      (print-string (vector-get edit0 4))
      (print-string "\n")
      (print (vector-get edit1 0))
      (print (vector-get edit1 1))
      (print (vector-get edit1 2))
      (print (vector-get edit1 3))
      (print (vector-get edit2 0))
      (print (vector-get edit2 1))
      (print (vector-get edit2 2))
      (print (vector-get edit2 3))
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_rename", harness);

    assert_eq!(
        lines[0], "1",
        "rename changes は単一 URI 変更 1 件であるべき"
    );
    assert_eq!(
        lines[1], "2",
        "rename change は [uri, edits] の 2 要素であるべき"
    );
    assert_eq!(lines[2], "99", "rename change は元の uri を保持するべき");
    assert_eq!(
        lines[3], "3",
        "rename edits は定義 + 2 参照の合計 3 件であるべき"
    );
    assert_eq!(lines[4], "1", "1 件目の edit start-line は定義行であるべき");
    assert_eq!(
        lines[5], "7",
        "1 件目の edit start-col は square 定義名の先頭であるべき"
    );
    assert_eq!(lines[6], "1", "1 件目の edit end-line は定義行であるべき");
    assert_eq!(
        lines[7], "13",
        "1 件目の edit end-col は square 終端であるべき"
    );
    assert_eq!(
        lines[8], "cube",
        "1 件目の edit newText は新しい名前であるべき"
    );
    assert_eq!(
        lines[9], "2",
        "2 件目の edit start-line は 2 行目であるべき"
    );
    assert_eq!(
        lines[10], "16",
        "2 件目の edit start-col は最初の呼び出しであるべき"
    );
    assert_eq!(lines[11], "2", "2 件目の edit end-line は 2 行目であるべき");
    assert_eq!(
        lines[12], "22",
        "2 件目の edit end-col は最初の呼び出し終端であるべき"
    );
    assert_eq!(
        lines[13], "2",
        "3 件目の edit start-line は 2 行目であるべき"
    );
    assert_eq!(
        lines[14], "27",
        "3 件目の edit start-col は 2 つ目の呼び出しであるべき"
    );
    assert_eq!(lines[15], "2", "3 件目の edit end-line は 2 行目であるべき");
    assert_eq!(
        lines[16], "33",
        "3 件目の edit end-col は 2 つ目の呼び出し終端であるべき"
    );
}

/// TEST-LSP-20: completion が prefix とシンボル表から安定した item を返すこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_completion_uses_prefix_and_symbols() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src "(defn helper [] 1)\n(he)"
        params (vector-push (vector-push (vector-push (vector-push (vector-new 4) 77) 2) 4) src)
        items (handle-completion params state)
        item0 (vector-get items 0)]
    (do
      (print (vector-length items))
      (print (vector-length item0))
      (print-string (vector-get item0 0))
      (print-string "\n")
      (print (vector-get item0 1))
      (print-string (vector-get item0 2))
      (print-string "\n")
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_completion", harness);

    assert_eq!(
        lines[0], "1",
        "completion は prefix=he に対して helper のみ返すべき"
    );
    assert_eq!(
        lines[1], "3",
        "completion item は [label, kind, insertText] の 3 要素であるべき"
    );
    assert_eq!(lines[2], "helper", "completion label は helper であるべき");
    assert_eq!(lines[3], "3", "completion kind は関数 (3) であるべき");
    assert_eq!(
        lines[4], "helper",
        "completion insertText は helper であるべき"
    );
}

/// TEST-LSP-21: formatting が実ソース長に基づく full-document edit を返すこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_formatting_returns_document_edit() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src "(defn main []\n 1)"
        params (vector-push (vector-push (vector-new 2) 77) src)
        edits (handle-formatting params state)
        edit (vector-get edits 0)]
    (do
      (print (vector-length edits))
      (print (vector-get edit 0))
        (print (vector-get edit 1))
        (print (vector-get edit 2))
        (print (vector-get edit 3))
        (print-string (vector-get edit 4))
        0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_formatting", harness);

    assert_eq!(lines[0], "1", "formatting は 1 件の TextEdit を返すべき");
    assert_eq!(lines[1], "1", "TextEdit start-line は 1 であるべき");
    assert_eq!(lines[2], "1", "TextEdit start-col は 1 であるべき");
    assert_eq!(
        lines[3], "2",
        "TextEdit end-line は入力全文の終端であるべき"
    );
    assert_eq!(lines[4], "4", "TextEdit end-col は入力全文の終端であるべき");
    assert_eq!(
        lines[5], "(defn main [] 1)",
        "TextEdit newText は整形後の全文であるべき"
    );
}

/// TEST-LSP-21b: formatting が string literal を source-aware formatter 経由で返すこと
#[test]
fn test_e2e_selfhost_lsp_real_shapes_formatting_preserves_string_literal() {
    let harness = r#"
(defn main []
  (let [state (server-state-new)
        src "(defn main [] \"abc\")"
        params (vector-push (vector-push (vector-new 2) 77) src)
        edits (handle-formatting params state)
        edit (vector-get edits 0)]
    (do
      (print-string (vector-get edit 4))
      0)))
"#;

    let lines = run_lsp_harness("lsp_real_shapes_formatting_string_literal", harness);

    assert_eq!(
        lines[0], "(defn main [] \"abc\")",
        "LSP formatting は string literal を source-aware formatter で保持するべき"
    );
}

/// TEST-FMT-01: selfhost/Formatter.ls に format-program / format-expr 関数が存在すること
///
/// T4c-1 AC-300: parse-format-parse roundtrip のための format-program / format-expr
/// Red Phase: Formatter.ls に format-program / format-expr が未定義のため FAIL する。
#[test]
fn test_e2e_selfhost_formatter_roundtrip_v2() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fmt_path = project_root.join("selfhost/Formatter.ls");
    assert!(
        fmt_path.exists(),
        "selfhost/Formatter.ls が存在しない (T4-3)"
    );
    let source =
        std::fs::read_to_string(&fmt_path).expect("selfhost/Formatter.ls の読み込みに失敗");

    // T4c-1 AC-300: parse-format-parse roundtrip
    // format-program と format-expr (または同等関数) が定義されていること
    assert!(
        source.contains("format-program") || source.contains("format_program"),
        "selfhost/Formatter.ls に format-program 関数がない (AC-300)"
    );
    assert!(
        source.contains("format-expr") || source.contains("format_expr"),
        "selfhost/Formatter.ls に format-expr 関数がない (AC-300)"
    );
}

/// TEST-LINT-01: selfhost/Linter.ls に L0001 形式の rule ID が定義されていること
///
/// T4c-2 AC-304: 各 lint rule に一意の rule id (L0001 形式) が付与されている
/// Red Phase: Linter.ls に L0001 形式の rule ID が未定義のため FAIL する。
#[test]
fn test_e2e_selfhost_linter_rule_ids_v2() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lint_path = project_root.join("selfhost/Linter.ls");
    assert!(lint_path.exists(), "selfhost/Linter.ls が存在しない (T4-3)");
    let source = std::fs::read_to_string(&lint_path).expect("selfhost/Linter.ls の読み込みに失敗");

    // T4c-2 AC-304: 各 lint rule に一意の rule id (L0001 形式) が付与されている
    // L + 4桁の数字パターンを手動検索
    let has_rule_id = source.lines().any(|line| {
        let bytes = line.as_bytes();
        for i in 0..bytes.len().saturating_sub(4) {
            if bytes[i] == b'L'
                && i + 4 < bytes.len()
                && bytes[i + 1].is_ascii_digit()
                && bytes[i + 2].is_ascii_digit()
                && bytes[i + 3].is_ascii_digit()
                && bytes[i + 4].is_ascii_digit()
            {
                return true;
            }
        }
        false
    });
    assert!(
        has_rule_id,
        "selfhost/Linter.ls に L0001 形式の rule ID がない (AC-304)"
    );
}

/// TEST-DOC-01: docs/schemas/ に JSON schema ファイルが存在すること
///
/// T4d-1 AC-400/AC-401/AC-402: knowledge/review/doc の JSON Schema が docs/schemas/ に配置
/// Red Phase: docs/schemas/ ディレクトリが未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_doc_schemas() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let schemas_dir = project_root.join("docs/schemas");
    assert!(
        schemas_dir.exists() && schemas_dir.is_dir(),
        "docs/schemas/ ディレクトリが存在しない (T4d-1 AC-400)"
    );

    // AC-400: knowledge JSON の JSON Schema
    // AC-401: review output の JSON Schema
    // AC-402: doc generator の出力 schema
    let entries: Vec<_> = std::fs::read_dir(&schemas_dir)
        .expect("docs/schemas/ の読み込みに失敗")
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.ends_with(".json") || name.ends_with(".schema.json")
        })
        .collect();

    assert!(
        !entries.is_empty(),
        "docs/schemas/ に JSON schema ファイルが存在しない (AC-400/AC-401/AC-402)"
    );

    // 最低限 knowledge / review / doc の 3 schema が必要
    let schema_names: Vec<String> = entries
        .iter()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let required_schemas = ["knowledge", "review", "doc"];
    for schema in &required_schemas {
        let found = schema_names.iter().any(|n| n.contains(schema));
        assert!(
            found,
            "docs/schemas/ に '{}' 関連の schema がない (AC-400/AC-401/AC-402). 存在するファイル: {:?}",
            schema, schema_names
        );
    }
}

/// TEST-DOC-02: selfhost/DocTools.ls + HtmlDoc.ls が存在し deterministic HTML 生成に対応
///
/// T4d-3 AC-408/AC-409: deterministic 出力、タイムスタンプ非埋め込み
/// Red Phase: selfhost/DocTools.ls, HtmlDoc.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_doc_deterministic_html() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // DocTools.ls の存在確認 (T4d-3)
    let doctools_path = project_root.join("selfhost/DocTools.ls");
    assert!(
        doctools_path.exists(),
        "selfhost/DocTools.ls が存在しない (T4d-3: HTML doc 生成)"
    );

    // HtmlDoc.ls の存在確認
    let htmldoc_path = project_root.join("selfhost/HtmlDoc.ls");
    assert!(
        htmldoc_path.exists(),
        "selfhost/HtmlDoc.ls が存在しない (T4d-3: HTML doc 生成)"
    );

    let doctools_source =
        std::fs::read_to_string(&doctools_path).expect("selfhost/DocTools.ls の読み込みに失敗");
    let htmldoc_source =
        std::fs::read_to_string(&htmldoc_path).expect("selfhost/HtmlDoc.ls の読み込みに失敗");

    // module 宣言の存在確認
    assert!(
        doctools_source.contains("(module DocTools)") || doctools_source.contains("(module Doc"),
        "selfhost/DocTools.ls に module 宣言がない"
    );
    assert!(
        htmldoc_source.contains("(module HtmlDoc)") || htmldoc_source.contains("(module Html"),
        "selfhost/HtmlDoc.ls に module 宣言がない"
    );

    // doc 生成関数の存在確認
    assert!(
        doctools_source.contains("generate")
            || doctools_source.contains("gen-doc")
            || doctools_source.contains("doc-generate"),
        "selfhost/DocTools.ls に doc 生成関数がない"
    );
}

/// TEST-DOC-03: selfhost/DocTools.ls が top-level defn を公開関数として抽出できること
#[test]
fn test_e2e_selfhost_doctools_extract_public_functions_runtime() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] 42) (defn add [x y] (+ x y))")
        entries (extract-public-functions program)]
    (do
      (print (vector-length entries))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["2"], "public defn 2 件を抽出できるべき");
}

/// TEST-DOC-04: selfhost/DocTools.ls が type/type-alias を抽出できること
#[test]
fn test_e2e_selfhost_doctools_extract_type_definitions_runtime() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(type Foo Int) (type-alias Bar Int)")
        entries (extract-type-definitions program)]
    (do
      (print (vector-length entries))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["2"], "type 定義 2 件を抽出できるべき");
}

/// TEST-DOC-05: selfhost/DocTools.ls が module body の公開 defn を抽出できること
#[test]
fn test_e2e_selfhost_doctools_extract_module_public_functions_runtime() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (defn visible [] 1) (private (defn hidden [] 0)))")
        entries (extract-public-functions program)]
    (do
      (print (vector-length entries))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1"],
        "module body の公開 defn だけを抽出できるべき"
    );
}

/// TEST-DOC-06: selfhost/DocTools.ls が module body の type 宣言を抽出できること
#[test]
fn test_e2e_selfhost_doctools_extract_module_type_definitions_runtime() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(module Demo (type Thing Int) (type-alias Alias Int))")
        entries (extract-type-definitions program)]
    (do
      (print (vector-length entries))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_doctools_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["2"],
        "module body の type 系宣言を抽出できるべき"
    );
}

/// TEST-PKG-01: scripts/ に配布物作成スクリプトが存在すること
///
/// T4e-1/T4e-2: OS 別配布形式の固定 + release artifact の同梱物
/// Red Phase: 配布物作成スクリプトが未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_pkg_archives() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scripts_dir = project_root.join("scripts");
    assert!(
        scripts_dir.exists() && scripts_dir.is_dir(),
        "scripts/ ディレクトリが存在しない"
    );

    // T4e-1: OS 別配布形式の固定
    // T4e-2: release artifact の同梱物
    let entries: Vec<String> = std::fs::read_dir(&scripts_dir)
        .expect("scripts/ の読み込みに失敗")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // 配布物作成に関連するスクリプト (release / package / dist / archive)
    let has_pkg_script = entries.iter().any(|n| {
        n.contains("release")
            || n.contains("package")
            || n.contains("dist")
            || n.contains("archive")
    });
    assert!(
        has_pkg_script,
        "scripts/ に配布物作成スクリプト (release/package/dist/archive) がない (T4e-1). 存在するファイル: {:?}",
        entries
    );

    // checksums 生成スクリプトの存在確認 (AC-505: SHA-256 ハッシュ)
    let has_checksum_script = entries
        .iter()
        .any(|n| n.contains("checksum") || n.contains("sha256"));
    assert!(
        has_checksum_script,
        "scripts/ に checksum 生成スクリプトがない (AC-505). 存在するファイル: {:?}",
        entries
    );
}

/// GC-05 進捗: 同一ミニプログラムを短いループで compile+run（長寿命 soak の縮小版・CI 負荷を抑える）
#[test]
fn test_e2e_gc_light_compile_run_loop() {
    let src = r#"(defn main [] (print 1))"#;
    for _ in 0..48 {
        let out = compile_and_run(src);
        assert_eq!(out.trim(), "1", "GC light loop: 毎回同一出力");
    }
}

/// GC-05 拡張: 1000 回 compile+run ループ (runtime-stability-spec S8 の CI 簡易モード相当)
/// 通常テストでは skip し、CI で --include-ignored 実行する
#[test]
#[ignore]
fn test_e2e_gc_compile_run_loop_1000() {
    let src = r#"(defn main [] (print 1))"#;
    for i in 0..1000 {
        let out = compile_and_run(src);
        assert_eq!(
            out.trim(),
            "1",
            "GC 1000-loop: iteration {} で出力が不一致",
            i
        );
    }
}

/// GC-05: REPL セッション模擬 — 50 回の eval ループ（各 eval でメモリ確保）
/// 通常 CI 向けの軽量版。ループ内で毎回 alloc し、結果の決定性を検証。
#[test]
fn test_e2e_gc_repl_soak_50_eval() {
    // 50 回の alloc ループで最終アドレスが決定的であることを検証
    let src = r#"
        (defn eval-loop [n total]
          (if (<= n 0)
            total
            (let [addr (__alloc 32)]
              (eval-loop (- n 1) (+ total 1)))))
        (defn main []
          (let [result (eval-loop 50 0)]
            (do (print result) 0)))
    "#;
    let out = compile_and_run(src);
    assert_eq!(out.trim(), "50", "50 eval REPL soak: 全 eval が完了すべき");
}

/// GC-05: REPL セッション模擬 — 500 回の eval ループ（各 eval でメモリ確保）
/// Nightly / 手動実行向けの完全版。メモリ破損なく 500 eval を完走することを検証。
#[test]
#[ignore]
fn test_e2e_gc_repl_soak_500_eval() {
    // 500 回の alloc ループで最終カウントが正確であることを検証
    let src = r#"
        (defn eval-loop [n total]
          (if (<= n 0)
            total
            (let [addr (__alloc 32)]
              (eval-loop (- n 1) (+ total 1)))))
        (defn main []
          (let [result (eval-loop 500 0)]
            (do (print result) 0)))
    "#;
    let out = compile_and_run(src);
    assert_eq!(
        out.trim(),
        "500",
        "500 eval REPL soak: 全 eval が完了すべき"
    );
}

// ============================================================
// Group M: CI/Ops 系テスト (TEST-META-05, TEST-OPS-01〜08)
// ============================================================

/// TEST-META-05: tests/differential-allowlist.yaml の存在 + 構造検証
#[test]
fn test_e2e_meta05_differential_allowlist() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let allowlist_path = project_root.join("tests/differential-allowlist.yaml");
    assert!(
        allowlist_path.exists(),
        "tests/differential-allowlist.yaml が存在しない"
    );
    let content = std::fs::read_to_string(&allowlist_path)
        .expect("differential-allowlist.yaml の読み込みに失敗");
    // YAML として最低限のキーが含まれていること
    assert!(
        content.contains("allowlist"),
        "differential-allowlist.yaml に 'allowlist' キーが含まれていない: {}",
        content
    );
    // META-05: 許容エントリは空運用（エントリ追加は差分ゼロ不能時のみ）
    assert!(
        content.contains("allowlist: []"),
        "differential-allowlist.yaml は空配列 allowlist: [] を維持すること (META-05): {}",
        content
    );
}

/// TEST-OPS-01: .github/workflows/ci.yml に gate-v2 ジョブ構造 + ジョブグラフドキュメント
#[test]
fn test_e2e_ops01_ci_gate_v2() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let content = std::fs::read_to_string(&ci_path).expect("ci.yml の読み込みに失敗");
    // gate-v2 ジョブまたは ci-gate-v2 ジョブが存在すること
    assert!(
        content.contains("ci-gate-v2") || content.contains("gate-v2"),
        "ci.yml に gate-v2 / ci-gate-v2 ジョブが存在しない"
    );
    // ジョブグラフドキュメントが存在すること
    let job_graph_doc = project_root.join("docs/development/operations/ci-gate-v2-job-graph.md");
    assert!(
        job_graph_doc.is_file(),
        "docs/development/operations/ci-gate-v2-job-graph.md が存在しない"
    );
}

/// TEST-OPS-02: ci.yml に artifact retention 設定 + ポリシードキュメント
#[test]
fn test_e2e_ops02_artifact_policy() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let content = std::fs::read_to_string(&ci_path).expect("ci.yml の読み込みに失敗");
    // artifact retention に関する設定が存在すること
    assert!(
        content.contains("retention-days"),
        "ci.yml に artifact retention-days 設定が存在しない"
    );
    // アーティファクトポリシードキュメントが存在すること
    let policy_doc = project_root.join("docs/development/operations/artifact-policy.md");
    assert!(
        policy_doc.is_file(),
        "docs/development/operations/artifact-policy.md が存在しない"
    );
}

/// TEST-OPS-03: ci.yml に shadow/oracle ジョブ
#[test]
fn test_e2e_ops03_shadow_oracle() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let content = std::fs::read_to_string(&ci_path).expect("ci.yml の読み込みに失敗");
    // shadow または oracle ジョブが存在すること
    assert!(
        content.contains("shadow") || content.contains("oracle"),
        "ci.yml に shadow/oracle ジョブが存在しない"
    );
}

/// TEST-GC-06: GC metrics artifact script / workflow / docs が揃っていること
#[test]
fn test_e2e_gc06_ci_artifact_contract() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script = project_root.join("scripts/ci/collect-gc-metrics.sh");
    assert!(
        script.is_file(),
        "scripts/ci/collect-gc-metrics.sh が存在しない"
    );

    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.is_file(), "ci.yml が存在しない");
    let ci_content = std::fs::read_to_string(&ci_path).expect("ci.yml の読み込みに失敗");
    assert!(
        ci_content.contains("gc-metrics-artifact"),
        "ci.yml に gc-metrics-artifact ジョブが存在しない"
    );
    assert!(
        ci_content.contains("collect-gc-metrics.sh"),
        "ci.yml は collect-gc-metrics.sh を実行すること"
    );
    assert!(
        ci_content.contains("gc-metrics-"),
        "ci.yml は gc-metrics artifact 名を保持すること"
    );

    let policy = project_root.join("docs/development/operations/artifact-policy.md");
    assert!(policy.is_file(), "artifact-policy.md が存在しない");
    let policy_content =
        std::fs::read_to_string(&policy).expect("artifact-policy.md の読み込みに失敗");
    assert!(
        policy_content.contains("GC metrics"),
        "artifact-policy.md に GC metrics artifact 規則が存在しない"
    );

    let spec = project_root.join("docs/development/planning/gc-ci-gate-spec.md");
    assert!(spec.is_file(), "gc-ci-gate-spec.md が存在しない");
    let spec_content = std::fs::read_to_string(&spec).expect("gc-ci-gate-spec.md の読み込みに失敗");
    assert!(
        spec_content.contains("collect-gc-metrics.sh"),
        "gc-ci-gate-spec.md は collect-gc-metrics.sh を参照すること"
    );
}

/// TEST-OPS-04: legacy-rust-bootstrap/ ディレクトリ構造
#[test]
fn test_e2e_ops04_legacy_isolation() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let legacy_dir = project_root.join("legacy-rust-bootstrap");
    assert!(
        legacy_dir.exists() && legacy_dir.is_dir(),
        "legacy-rust-bootstrap/ ディレクトリが存在しない"
    );
    // README.md が含まれていること
    let readme = legacy_dir.join("README.md");
    assert!(
        readme.exists(),
        "legacy-rust-bootstrap/README.md が存在しない"
    );
}

/// TEST-OPS-05: driver/main.rs に L# path 設定
#[test]
fn test_e2e_ops05_default_path_migration() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main_rs = project_root.join("crates/lsharp-driver/src/main.rs");
    assert!(main_rs.exists(), "main.rs が存在しない");
    let content = std::fs::read_to_string(&main_rs).expect("main.rs の読み込みに失敗");
    // L# compiler path に関する設定またはコメントが存在すること
    assert!(
        content.contains("LSHARP_PATH")
            || content.contains("lsharp_path")
            || content.contains("compiler path"),
        "main.rs に L# compiler path 設定が存在しない"
    );
    let smoke = project_root.join("scripts/ci/default-path-smoke.sh");
    assert!(
        smoke.is_file(),
        "scripts/ci/default-path-smoke.sh が存在しない (OPS-05 CI gate)"
    );
    let smoke_content =
        std::fs::read_to_string(&smoke).expect("default-path-smoke.sh の読み込みに失敗");
    assert!(
        smoke_content.contains("LSHARP_PATH"),
        "default-path-smoke.sh は LSHARP_PATH delegation hook も検証すること"
    );
    assert!(
        smoke_content.contains("delegated-exec:--version"),
        "default-path-smoke.sh は executable path delegation を smoke すること"
    );
    assert!(
        smoke_content.contains("delegated-dir:--version"),
        "default-path-smoke.sh は directory path delegation を smoke すること"
    );
    let doc = project_root.join("docs/development/operations/default-path-migration.md");
    assert!(
        doc.is_file(),
        "docs/development/operations/default-path-migration.md が存在しない"
    );
    let doc_content =
        std::fs::read_to_string(&doc).expect("default-path-migration.md の読み込みに失敗");
    assert!(
        doc_content.contains("default_path_delegation"),
        "default-path-migration.md は delegation test の証跡を記載すること"
    );
    assert!(
        doc_content.contains("process-entry delegation"),
        "default-path-migration.md は LSHARP_PATH が process-entry delegation であることを明記すること"
    );
    assert!(
        doc_content.contains("13 CLI サブコマンド"),
        "default-path-migration.md は公開 command surface 全体を明記すること"
    );
    let matrix = project_root.join("docs/development/planning/compatibility-matrix.md");
    assert!(
        matrix.is_file(),
        "docs/development/planning/compatibility-matrix.md が存在しない"
    );
    let matrix_content =
        std::fs::read_to_string(&matrix).expect("compatibility-matrix.md の読み込みに失敗");
    assert!(
        matrix_content.contains("Default path / delegation サマリ"),
        "compatibility-matrix.md は default path / delegation サマリを持つこと"
    );
    assert!(
        matrix_content.contains("argv 丸ごと外部 `lsharp` binary へ委譲"),
        "compatibility-matrix.md は LSHARP_PATH の argv delegation を明記すること"
    );
}

/// TEST-OPS-06: scripts/ に release playbook + ドキュメント + tag push 自動化 workflow
#[test]
fn test_e2e_ops06_release_playbook() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scripts_dir = project_root.join("scripts");
    let entries: Vec<String> = std::fs::read_dir(&scripts_dir)
        .expect("scripts/ の読み込みに失敗")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let has_playbook = entries.iter().any(|n| n.contains("playbook"));
    assert!(
        has_playbook,
        "scripts/ に release playbook スクリプトが存在しない. 存在するファイル: {:?}",
        entries
    );
    // リリースプレイブックドキュメントが存在すること
    let playbook_doc = project_root.join("docs/development/operations/release-playbook.md");
    assert!(
        playbook_doc.is_file(),
        "docs/development/operations/release-playbook.md が存在しない"
    );
    // tag push による自動リリース workflow が存在すること (OPS-06 tag-push automation)
    let release_workflow = project_root.join(".github/workflows/release.yml");
    assert!(
        release_workflow.is_file(),
        ".github/workflows/release.yml が存在しない -- tag push 自動リリースが未実装"
    );
    // workflow が v* タグトリガーを含むこと
    let workflow_content =
        std::fs::read_to_string(&release_workflow).expect("release.yml の読み込みに失敗");
    assert!(
        workflow_content.contains("v*") || workflow_content.contains("'v"),
        "release.yml に v* タグトリガーが設定されていない"
    );
}

/// TEST-OPS-07: scripts/smoke_test_readme.sh の存在 + 実行可能 + fresh clone 仕様ドキュメント
#[test]
fn test_e2e_ops07_fresh_clone_no_rust() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let smoke_script = project_root.join("scripts/smoke_test_readme.sh");
    assert!(
        smoke_script.exists(),
        "scripts/smoke_test_readme.sh が存在しない"
    );
    // 実行可能ビットが設定されていること (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta =
            std::fs::metadata(&smoke_script).expect("smoke_test_readme.sh のメタデータ取得失敗");
        let mode = meta.permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "scripts/smoke_test_readme.sh に実行可能ビットがない (mode: {:o})",
            mode
        );
    }
    // fresh clone 仕様ドキュメントが存在すること
    let fresh_clone_doc = project_root.join("docs/development/operations/fresh-clone-spec.md");
    assert!(
        fresh_clone_doc.is_file(),
        "docs/development/operations/fresh-clone-spec.md が存在しない"
    );
}

/// TEST-OPS-08: scripts/ に rollback スクリプト + docs/ に手順 + 撤去 ADR
#[test]
fn test_e2e_ops08_final_removal_rollback() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // rollback スクリプトの存在
    let scripts_dir = project_root.join("scripts");
    let entries: Vec<String> = std::fs::read_dir(&scripts_dir)
        .expect("scripts/ の読み込みに失敗")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let has_rollback = entries.iter().any(|n| n.contains("rollback"));
    assert!(
        has_rollback,
        "scripts/ に rollback スクリプトが存在しない. 存在するファイル: {:?}",
        entries
    );

    // docs/ にロールバック手順ドキュメント
    let docs_dir = project_root.join("docs");
    assert!(
        docs_dir.exists() && docs_dir.is_dir(),
        "docs/ ディレクトリが存在しない"
    );
    let rollback_candidates = [
        project_root.join("docs/rollback-procedure.md"),
        project_root.join("docs/development/operations/rollback-procedure.md"),
    ];
    let has_rollback_doc = rollback_candidates.iter().any(|p| p.is_file());
    assert!(
        has_rollback_doc,
        "rollback 手順ドキュメントが見つからない (期待: {:?})",
        rollback_candidates
    );

    // Rust 撤去 ADR ドキュメントが存在すること
    let adr_doc = project_root.join("docs/development/operations/adr-rust-removal.md");
    assert!(
        adr_doc.is_file(),
        "docs/development/operations/adr-rust-removal.md が存在しない"
    );
}

/// TEST-OPS-09: fresh clone smoke ジョブが clean checkout + binary path を検証する
#[test]
fn test_fresh_clone_smoke_ci_job() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let ci_content = std::fs::read_to_string(&ci_path).expect("ci.yml の読み込みに失敗");
    assert!(
        ci_content.contains("fresh-clone-smoke"),
        "ci.yml に fresh-clone-smoke ジョブが存在しない"
    );
    assert!(
        ci_content.contains("bash scripts/ci/test-fresh-clone.sh"),
        "ci.yml が scripts/ci/test-fresh-clone.sh を実行していない"
    );

    let smoke_script = project_root.join("scripts/ci/test-fresh-clone.sh");
    assert!(
        smoke_script.is_file(),
        "scripts/ci/test-fresh-clone.sh が存在しない"
    );
    let script_content =
        std::fs::read_to_string(&smoke_script).expect("test-fresh-clone.sh の読み込みに失敗");
    assert!(
        script_content.contains("git archive"),
        "test-fresh-clone.sh は clean checkout を git archive で再現すること"
    );
    assert!(
        script_content.contains("default-path-smoke.sh"),
        "test-fresh-clone.sh は既存の default-path-smoke.sh を再利用すること"
    );
    assert!(
        script_content.contains("selfhost/Token.ls") || script_content.contains("stdlib/Core.ls"),
        "test-fresh-clone.sh は clean checkout 上で selfhost/stdlib の実コンパイルを行うこと"
    );

    let fresh_clone_doc = project_root.join("docs/development/operations/fresh-clone-spec.md");
    let doc_content =
        std::fs::read_to_string(&fresh_clone_doc).expect("fresh-clone-spec.md の読み込みに失敗");
    assert!(
        doc_content.contains("fresh-clone-smoke"),
        "fresh-clone-spec.md に現行の fresh-clone-smoke ジョブを記載すること"
    );
}

/// TEST-OPS-10: phase11 compile gate は cargo run ではなくビルド済み lsharp バイナリを使う
#[test]
fn test_phase11_compile_gate_uses_lsharp_binary() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let script_path = project_root.join("scripts/ci/compile-phase11-inputs.sh");
    assert!(
        script_path.is_file(),
        "scripts/ci/compile-phase11-inputs.sh が存在しない"
    );
    let content =
        std::fs::read_to_string(&script_path).expect("compile-phase11-inputs.sh の読み込みに失敗");
    assert!(
        content.contains("LSHARP_BIN"),
        "compile-phase11-inputs.sh は LSHARP_BIN を受け取れること"
    );
    assert!(
        content.contains("\"$LSHARP_BIN\" compile")
            || content.contains("\"${LSHARP_BIN}\" compile"),
        "compile-phase11-inputs.sh はビルド済み lsharp バイナリで compile を実行すること"
    );
}

fn formatter_output_for_source(source: &str) -> String {
    assert!(
        !source.contains('"'),
        "formatter_output_for_source は引用符を含まない source 専用 helper"
    );

    let harness = format!(
        r#"
(defn main []
  (do
    (print-string (format-program (parse-program "{source}") 0))
    0))
"#
    );

    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    compile_and_run(&combined)
}

fn selfhost_formatter_runtime_bundle() -> String {
    [
        "Token.ls",
        "AST.ls",
        "Lexer.ls",
        "Parser.ls",
        "Formatter.ls",
    ]
    .into_iter()
    .map(selfhost_module)
    .collect::<Vec<_>>()
    .join("\n")
}

/// D-2: Formatter.ls の format-expr が lit-int AST ノードを実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_expr_lit_int() {
    let harness = r#"
(defn main []
  (let [node (vector-push (vector-push (vector-new 2) 1) 42)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"42",
        "lit-int 42 は文字列 \"42\" に整形されるべき"
    );
}

/// D-2: format-expr が apply AST ノードを実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_expr_apply() {
    let harness = r#"
(defn main []
  (let [func-node (vector-push (vector-push (vector-new 2) 4) 102)
        arg1 (vector-push (vector-push (vector-new 2) 1) 1)
        arg2 (vector-push (vector-push (vector-new 2) 1) 2)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 5) func-node) 2) arg1) arg2)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines.last().unwrap(),
        &"(f 1 2)",
        "apply は実テキスト \"(f 1 2)\" を返すべき"
    );
}

/// FMT-01: format-expr が let (tag=7) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_expr_let() {
    let harness = r#"
(defn main []
  (let [init-expr (vector-push (vector-push (vector-new 2) 1) 10)
        body-expr (vector-push (vector-push (vector-new 2) 4) 120)
        node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 120) init-expr) body-expr)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(let [x 10] x)",
        "let は実テキストへ整形されるべき"
    );
}

/// FMT-01: format-expr が if (tag=6) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_expr_if() {
    let harness = r#"
(defn main []
  (let [cond-expr (vector-push (vector-push (vector-new 2) 4) 120)
        then-expr (vector-push (vector-push (vector-new 2) 1) 42)
        else-expr (vector-push (vector-push (vector-new 2) 1) 0)
        node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) cond-expr) then-expr) else-expr)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(if x 42 0)",
        "if は実テキストへ整形されるべき"
    );
}

/// FMT-01: format-expr が lambda (tag=8) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_expr_lambda() {
    let harness = r#"
(defn main []
  (let [func-node (vector-push (vector-push (vector-new 2) 4) 102)
        arg1 (vector-push (vector-push (vector-new 2) 4) 120)
        arg2 (vector-push (vector-push (vector-new 2) 4) 121)
        body-expr (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 5) func-node) 2) arg1) arg2)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 8) 2) 120) 121) body-expr)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(fn [x y] (f x y))",
        "lambda は実テキストへ整形されるべき"
    );
}

/// FMT-01: format-expr が do (tag=9) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_expr_do() {
    let harness = r#"
(defn main []
  (let [e1 (vector-push (vector-push (vector-new 2) 1) 1)
        e2 (vector-push (vector-push (vector-new 2) 1) 2)
        e3 (vector-push (vector-push (vector-new 2) 1) 3)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 9) 3) e1) e2) e3)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(do 1 2 3)",
        "do は実テキストへ整形されるべき"
    );
}

/// FMT-01: match が canonical な実テキストへ整形されること
#[test]
fn test_e2e_selfhost_formatter_format_expr_match() {
    let harness = r#"
(defn main []
  (let [scr (vector-push (vector-push (vector-new 2) 4) 120)
        lit1 (vector-push (vector-push (vector-new 2) 1) 1)
        pat1 (vector-push (vector-push (vector-new 2) 42) lit1)
        body1 (vector-push (vector-push (vector-new 2) 1) 10)
        lit2 (vector-push (vector-push (vector-new 2) 1) 2)
        pat2 (vector-push (vector-push (vector-new 2) 42) lit2)
        body2 (vector-push (vector-push (vector-new 2) 1) 20)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 10) scr) 2) pat1) body1) pat2) body2)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(match x [1 10] [2 20])",
        "match は canonical text を返すべき"
    );
}

/// FMT-01: recordlit が canonical な実テキストへ整形されること
#[test]
fn test_e2e_selfhost_formatter_format_expr_recordlit() {
    let harness = r#"
(defn main []
  (let [f1-expr (vector-push (vector-push (vector-new 2) 1) 1)
        f2-expr (vector-push (vector-push (vector-new 2) 1) 2)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 12) 80) 2) 120) f1-expr) 121) f2-expr)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"{P x 1 y 2}",
        "recordlit は canonical text を返すべき"
    );
}

/// FMT-01: fieldaccess が canonical な実テキストへ整形されること
#[test]
fn test_e2e_selfhost_formatter_format_expr_fieldaccess() {
    let harness = r#"
(defn main []
  (let [inner (vector-push (vector-push (vector-new 2) 4) 120)
        node (vector-push (vector-push (vector-push (vector-new 3) 13) inner) 121)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(. x y)",
        "fieldaccess は canonical text を返すべき"
    );
}

/// FMT-01: recordupdate が canonical な実テキストへ整形されること
#[test]
fn test_e2e_selfhost_formatter_format_expr_recordupdate() {
    let harness = r#"
(defn main []
  (let [base (vector-push (vector-push (vector-new 2) 4) 112)
        x-expr (vector-push (vector-push (vector-new 2) 1) 10)
        y-expr (vector-push (vector-push (vector-new 2) 1) 20)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 14) base) 2) 120) x-expr) 121) y-expr)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"{p | x 10 y 20}",
        "recordupdate は canonical text を返すべき"
    );
}

/// FMT-01: computation が canonical な実テキストへ整形されること
#[test]
fn test_e2e_selfhost_formatter_format_expr_computation() {
    let harness = r#"
(defn main []
  (let [step1-expr (vector-push (vector-push (vector-new 2) 4) 102)
        step2-expr (vector-push (vector-push (vector-new 2) 4) 121)
        step3-expr (vector-push (vector-push (vector-new 2) 4) 122)
        step4-expr (vector-push (vector-push (vector-new 2) 4) 120)
        node (vector-push
               (vector-push
                 (vector-push
                   (vector-push
                     (vector-push
                       (vector-push
                         (vector-push
                           (vector-push
                             (vector-push
                               (vector-push
                                 (vector-push
                                   (vector-push
                                     (vector-push
                                       (vector-push (vector-new 15) 15)
                                       109)
                                     4)
                                   (computation-step-let-bang))
                                 120)
                               step1-expr)
                             (computation-step-do-bang))
                           0)
                         step2-expr)
                       (computation-step-expr))
                     0)
                   step3-expr)
                 (computation-step-return))
               0)
        node (vector-push node step4-expr)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(computation m (let! x f) (do! y) z (return x))",
        "computation は canonical text を返すべき"
    );
}

/// FMT-01: source なしの string literal fallback が canonical text を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_lit_string_fallback() {
    let harness = r#"
(defn main []
  (let [node (vector-push (vector-push (vector-push (vector-new 3) 3) 10) 12)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"\"\"",
        "source なし string literal fallback は空文字 literal を返すべき"
    );
}

/// FMT-01: source なしの float literal fallback が canonical text を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_lit_float_fallback() {
    let harness = r#"
(defn main []
  (let [node (vector-push (vector-push (vector-push (vector-new 3) 19) 10) 13)
        result (format-expr node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"0.0",
        "source なし float literal fallback は 0.0 を返すべき"
    );
}

/// FMT-01: format-decl が defn (tag=20) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_decl_defn() {
    let harness = r#"
(defn main []
  (let [body (vector-push (vector-push (vector-new 2) 1) 0)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 20) 97) 3) 120) 121) 122) body)
        result (format-decl node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(defn a [x y z] 0)",
        "defn は実テキストへ整形されるべき"
    );
}

/// FMT-01: format-decl が body 付き module (tag=25) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_decl_module_with_body() {
    let harness = r#"
(defn main []
  (let [body (vector-push (vector-push (vector-new 2) 26) 97)
        node (vector-push (vector-push (vector-push (vector-new 4) 25) 77) 1)
        node (vector-push node body)
        result (format-decl node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(module M (import a))",
        "module は body 付き実テキストへ整形されるべき"
    );
}

/// FMT-01: format-decl が computation-builder (tag=30) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_decl_computation_builder() {
    let harness = r#"
(defn main []
  (let [node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 30) 109) 98) 105)
        result (format-decl node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(computation-builder m b i)",
        "computation-builder は実テキストへ整形されるべき"
    );
}

/// FMT-01: format-decl が impl (tag=28) を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_decl_impl() {
    let harness = r#"
(defn main []
  (let [body (vector-push (vector-push (vector-new 2) 26) 97)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 28) 83) 73) 1) body)
        result (format-decl node 0)]
    (do
      (print-string result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_formatter_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines.last().unwrap(),
        &"(impl (S I) (import a))",
        "impl は実テキストへ整形されるべき"
    );
}

/// FMT-01: format-program が supported subset を実テキストへ整形すること
#[test]
fn test_e2e_selfhost_formatter_format_program_text_simple_program() {
    let source = "  (defn a [b] (if b (let [c 1] c) (do b 0)))  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(defn a [b] (if b (let [c 1] c) (do b 0)))\n",
        "format-program は simple defn/if/let/do を canonical な実テキストへ整形するべき"
    );

    let parsed = parse_for_pipeline(&output);
    assert_eq!(
        parsed.decls.len(),
        1,
        "format-program の出力は Rust parser でも再パースできるべき"
    );
}

/// FMT-01: format-program が recordupdate を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_recordupdate_expr() {
    let source = "  {p | x 10 y 20}  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "{p | x 10 y 20}\n",
        "format-program は recordupdate を canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が computation を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_computation_expr() {
    let source = "  (computation m (let! x f) (do! y) z (return x))  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(computation m (let! x f) (do! y) z (return x))\n",
        "format-program は computation を canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が computation-builder 宣言を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_computation_builder_decl() {
    let source = "  (computation-builder m b i)  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(computation-builder m b i)\n",
        "format-program は computation-builder 宣言を canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が impl 宣言を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_impl_decl() {
    let source = "  (impl (S I) (import a))  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(impl (S I) (import a))\n",
        "format-program は impl 宣言を canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が module body を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_module_decl() {
    let source = "  (module Demo (import Core))  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(module Demo (import Core))\n",
        "format-program は module body を canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が簡易 decl payload の type 系宣言を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_missing_type_family_decls() {
    let source = "  (type Point) (type Pair (record (: x Int) (: y Int))) (type-alias Str String) (type-constrained Natural Int :constraints [(>= 0)])  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output,
        "(type Point)\n(type Pair (record))\n(type-alias Str Str)\n(type-constrained Natural Natural)\n",
        "format-program は簡易 payload の type 系宣言を decl kind を保った canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が trait 宣言を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_trait_decl() {
    let source = "  (trait (Show a) (defn show [self] self))  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(trait (Show) (defn show [self] self))\n",
        "format-program は trait 宣言を canonical text へ整形するべき"
    );
}

/// FMT-01: format-program が defmacro 宣言を canonical text へ整形できること
#[test]
fn test_e2e_selfhost_formatter_format_program_defmacro_decl() {
    let source = "  (defmacro double [x] '(+ ~x ~x))  ";
    let output = formatter_output_for_source(source);

    assert_eq!(
        output, "(defmacro double [x] '(+ ~x ~x))\n",
        "format-program は defmacro 宣言を canonical text へ整形するべき"
    );
}

/// FMT-01 AC-300: parse(format(src)) 後に再 format しても同じ実テキストになること
#[test]
fn test_e2e_selfhost_formatter_roundtrip_deterministic() {
    let source = " (defn a [b] (if b (let [c 1] c) (do b 0))) ";
    let formatted = formatter_output_for_source(source);
    let reformatted = formatter_output_for_source(formatted.trim_end());

    assert_eq!(
        formatted, reformatted,
        "roundtrip: parse(format(src)) 後も format-program の実テキストは安定するべき"
    );
}

/// FMT-01 AC-301: supported subset の format-program は 2 回適用しても同じ出力になること
#[test]
fn test_e2e_selfhost_formatter_idempotent() {
    let source = " (defn a [] 42)   (defn b [c] (do c 0)) ";
    let first = formatter_output_for_source(source);
    let second = formatter_output_for_source(first.trim_end());

    assert_eq!(
        first, "(defn a [] 42)\n(defn b [c] (do c 0))\n",
        "format-program は複数 defn も改行区切りの実テキストへ整形するべき"
    );
    assert_eq!(
        first, second,
        "format-program は text output に対して idempotent であるべき"
    );
}
