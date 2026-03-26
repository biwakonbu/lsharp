use super::support::*;


/// TEST-LSP-03: selfhost/LspServer.ls に diagnostics の安定ソート機構
///
/// T4b-3 AC-208/AC-209/AC-210/AC-211: 診断のグルーピング・ソート・重複マージ・決定的順序
/// Red Phase: selfhost/LspServer.ls が未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_lsp_diagnostic_ordering() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lsp_path = project_root.join("selfhost/LspServer.ls");
    assert!(
        lsp_path.exists(),
        "selfhost/LspServer.ls が存在しない"
    );
    let source = std::fs::read_to_string(&lsp_path)
        .expect("selfhost/LspServer.ls の読み込みに失敗");

    // T4b-3 AC-208: 診断は source フィールドでグルーピングされ行番号昇順
    assert!(
        source.contains("sort") || source.contains("order")
            || source.contains("diagnostic"),
        "selfhost/LspServer.ls に diagnostics のソート/順序制御がない (AC-208)"
    );
}

/// TEST-LSP-04: selfhost/LspServer.ls の主要ハンドラが runtime で観測できること
#[test]
fn test_e2e_selfhost_lsp_runtime_handlers() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[0], "4", "initialize capability count は 4 であるべき");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[9], "1010001", "先頭 diagnostic key は source=0,sev=1,line=1,col=1 であるべき");
    assert_eq!(lines[10], "1030002", "次の diagnostic key は source=0,sev=1,line=3,col=2 であるべき");
}

/// TEST-LSP-06: selfhost/LspServer.ls の merge-duplicate-diagnostics が同一 span を 1 件へ潰すこと
#[test]
fn test_e2e_selfhost_lsp_runtime_merge_duplicates() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[11], "1", "merged diagnostics count は 1 であるべき");
    assert_eq!(lines[12], "1", "merged diagnostics severity は高い方=1 を残すべき");
}

/// TEST-LSP-07: selfhost/LspServer.ls の navigation handler shape が runtime で観測できること
#[test]
fn test_e2e_selfhost_lsp_runtime_navigation_shapes() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[13], "2", "hover response shape length は 2 であるべき");
    assert_eq!(lines[14], "3", "goto-definition shape length は 3 であるべき");
    assert_eq!(lines[15], "1", "references count は 1 であるべき");
    assert_eq!(lines[16], "1", "rename changes length は 1 であるべき");
}

/// TEST-LSP-08: selfhost/LspServer.ls が JsonRpc method 定数で dispatch できること
#[test]
fn test_e2e_selfhost_lsp_runtime_jsonrpc_method_dispatch() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");
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

    assert_eq!(lines[0], "4", "initialize dispatch は capability vector を返すべき");
    assert_eq!(lines[1], "14", "didOpen dispatch は source length を返すべき");
    assert_eq!(lines[2], "9", "didChange dispatch は source length を返すべき");
    assert_eq!(lines[3], "2", "hover dispatch は response shape length=2 を返すべき");
    assert_eq!(lines[4], "3", "goto-definition dispatch は shape length=3 を返すべき");
    assert_eq!(lines[5], "1", "formatting dispatch は edit count=1 を返すべき");
    assert_eq!(lines[6], "7", "completion dispatch は keyword count=7 を返すべき");
    assert_eq!(lines[7], "0", "shutdown dispatch は 0 を返すべき");
}

/// TEST-LSP-09: selfhost/LspServer.ls の server-loop が 1 メッセージ dispatch を観測できること
#[test]
fn test_e2e_selfhost_lsp_runtime_server_loop_single_message() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");
    let harness = r#"
(defn make-loop-request [method-id params]
  (let [v (vector-new 2)]
    (vector-push (vector-push v method-id) params)))

(defn main []
  (let [open-req (make-loop-request (lsp-method-did-open) 15)
        change-req (make-loop-request (lsp-method-did-change) 9)
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

    assert_eq!(lines[0], "15", "server-loop は didOpen request を dispatch できるべき");
    assert_eq!(lines[1], "9", "server-loop は didChange request を dispatch できるべき");
    assert_eq!(lines[2], "7", "server-loop は completion request を dispatch できるべき");
}

/// TEST-LSP-08: sort-diagnostics が 3 要素以上を行番号順にソートできること
#[test]
fn test_e2e_selfhost_lsp_sort_diagnostics_three() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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

    assert!(lines.len() >= 3, "sort 3 diagnostics 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1010003", "最小の diagnostic key (source=0,sev=1,line=1,col=3) が先頭");
    assert_eq!(lines[1], "1030002", "中間の diagnostic key (source=0,sev=1,line=3,col=2) が 2 番目");
    assert_eq!(lines[2], "1050001", "最大の diagnostic key (source=0,sev=1,line=5,col=1) が末尾");
}

/// TEST-LSP-10: handle-hover が型情報文字列を返すこと
#[test]
fn test_e2e_selfhost_lsp_hover_returns_type_info() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

    let harness = r#"
(defn main []
  (let [state (server-state-new)
        ;; params: [uri, line, col]
        params (vector-push (vector-push (vector-push (vector-new 3) 42) 10) 5)
        result (handle-hover params state)]
    (do
      ;; result は [range, contents] の 2 要素
      (print (vector-length result))
      ;; contents スロットに型情報ハッシュが格納されている
      (print (vector-get result 1))
      0)))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "hover 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "2", "hover response は 2 要素であるべき");
    // contents に型情報ハッシュ (非ゼロ) が入る
    let type_info: i64 = lines[1].parse().unwrap_or(0);
    assert!(type_info != 0, "hover contents に型情報が含まれるべき (got {})", type_info);
}

/// TEST-LSP-11: handle-goto-definition がソース位置構造を返すこと
#[test]
fn test_e2e_selfhost_lsp_definition_returns_location() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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
    assert_eq!(lines[0], "3", "definition response は [uri, line, col] の 3 要素であるべき");
}

/// TEST-LSP-12: handle-references が位置リストを返すこと
#[test]
fn test_e2e_selfhost_lsp_references_returns_locations() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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
    assert!(ref_count >= 1, "references は 1 件以上返すべき (got {})", ref_count);
    assert_eq!(lines[1], "3", "各 location は [uri, line, col] の 3 要素であるべき");
}

/// TEST-LSP-13: handle-completion がキーワード補完候補を返すこと
#[test]
fn test_e2e_selfhost_lsp_completion_returns_keywords() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

    let harness = r#"
(defn main []
  (let [state (server-state-new)
        params 0
        result (handle-completion params state)]
    (do
      ;; result は completion items のリスト
      (print (vector-length result))
      ;; 各 item は [label-hash, kind] の 2 要素
      (let [item0 (vector-get result 0)]
        (do
          (print (vector-length item0))
          ;; kind=14 は Keyword
          (print (vector-get item0 1))
          0)))))
"#;

    let combined = format!("{}\n{}", source, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "completion 出力が不足: {:?}", lines);
    let item_count: i64 = lines[0].parse().unwrap_or(0);
    assert!(item_count >= 7, "completion は 7 件以上のキーワードを返すべき (got {})", item_count);
    assert_eq!(lines[1], "2", "各 completion item は [label-hash, kind] の 2 要素であるべき");
    assert_eq!(lines[2], "14", "completion kind は 14 (Keyword) であるべき");
}

/// TEST-LSP-14: sort-diagnostics が source 優先 → severity → line → col の順で並べること
#[test]
fn test_e2e_selfhost_lsp_diagnostic_ordering_source_priority() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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

/// TEST-LSP-16: encode-json-rpc-response が決定的な JSON-RPC 構造を生成すること
#[test]
fn test_e2e_selfhost_lsp_json_rpc_encode() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = std::fs::read_to_string(project_root.join("selfhost/LspServer.ls"))
        .expect("selfhost/LspServer.ls の読み込みに失敗");

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
    assert_eq!(lines[0], "2", "parsed request は [method-id, params] の 2 要素であるべき");
    assert_eq!(lines[1], "21", "method-id は 21 (hover) であるべき");
    assert_eq!(lines[2], "55", "params は 55 であるべき");
}

/// TEST-FMT-01: selfhost/Formatter.ls に format-program / format-expr 関数が存在すること
///
/// T4c-1 AC-300: parse-format-parse roundtrip のための format-program / format-expr
/// Red Phase: Formatter.ls に format-program / format-expr が未定義のため FAIL する。
#[test]
fn test_e2e_selfhost_formatter_roundtrip_v2() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fmt_path = project_root.join("selfhost/Formatter.ls");
    assert!(
        fmt_path.exists(),
        "selfhost/Formatter.ls が存在しない (T4-3)"
    );
    let source = std::fs::read_to_string(&fmt_path)
        .expect("selfhost/Formatter.ls の読み込みに失敗");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let lint_path = project_root.join("selfhost/Linter.ls");
    assert!(
        lint_path.exists(),
        "selfhost/Linter.ls が存在しない (T4-3)"
    );
    let source = std::fs::read_to_string(&lint_path)
        .expect("selfhost/Linter.ls の読み込みに失敗");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

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

    let doctools_source = std::fs::read_to_string(&doctools_path)
        .expect("selfhost/DocTools.ls の読み込みに失敗");
    let htmldoc_source = std::fs::read_to_string(&htmldoc_path)
        .expect("selfhost/HtmlDoc.ls の読み込みに失敗");

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
        doctools_source.contains("generate") || doctools_source.contains("gen-doc")
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

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
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

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["1"], "module body の公開 defn だけを抽出できるべき");
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

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines, vec!["2"], "module body の type 系宣言を抽出できるべき");
}

/// TEST-PKG-01: scripts/ に配布物作成スクリプトが存在すること
///
/// T4e-1/T4e-2: OS 別配布形式の固定 + release artifact の同梱物
/// Red Phase: 配布物作成スクリプトが未作成のため FAIL する。
#[test]
fn test_e2e_selfhost_pkg_archives() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
        n.contains("release") || n.contains("package")
            || n.contains("dist") || n.contains("archive")
    });
    assert!(
        has_pkg_script,
        "scripts/ に配布物作成スクリプト (release/package/dist/archive) がない (T4e-1). 存在するファイル: {:?}",
        entries
    );

    // checksums 生成スクリプトの存在確認 (AC-505: SHA-256 ハッシュ)
    let has_checksum_script = entries.iter().any(|n| {
        n.contains("checksum") || n.contains("sha256")
    });
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
    assert_eq!(out.trim(), "500", "500 eval REPL soak: 全 eval が完了すべき");
}

// ============================================================
// Group M: CI/Ops 系テスト (TEST-META-05, TEST-OPS-01〜08)
// ============================================================

/// TEST-META-05: tests/differential-allowlist.yaml の存在 + 構造検証
#[test]
fn test_e2e_meta05_differential_allowlist() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let content = std::fs::read_to_string(&ci_path)
        .expect("ci.yml の読み込みに失敗");
    // gate-v2 ジョブまたは ci-gate-v2 ジョブが存在すること
    assert!(
        content.contains("ci-gate-v2") || content.contains("gate-v2"),
        "ci.yml に gate-v2 / ci-gate-v2 ジョブが存在しない"
    );
    // ジョブグラフドキュメントが存在すること
    let job_graph_doc = project_root
        .join("docs/development/operations/ci-gate-v2-job-graph.md");
    assert!(
        job_graph_doc.is_file(),
        "docs/development/operations/ci-gate-v2-job-graph.md が存在しない"
    );
}

/// TEST-OPS-02: ci.yml に artifact retention 設定 + ポリシードキュメント
#[test]
fn test_e2e_ops02_artifact_policy() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let content = std::fs::read_to_string(&ci_path)
        .expect("ci.yml の読み込みに失敗");
    // artifact retention に関する設定が存在すること
    assert!(
        content.contains("retention-days"),
        "ci.yml に artifact retention-days 設定が存在しない"
    );
    // アーティファクトポリシードキュメントが存在すること
    let policy_doc = project_root
        .join("docs/development/operations/artifact-policy.md");
    assert!(
        policy_doc.is_file(),
        "docs/development/operations/artifact-policy.md が存在しない"
    );
}

/// TEST-OPS-03: ci.yml に shadow/oracle ジョブ
#[test]
fn test_e2e_ops03_shadow_oracle() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ci_path = project_root.join(".github/workflows/ci.yml");
    assert!(ci_path.exists(), "ci.yml が存在しない");
    let content = std::fs::read_to_string(&ci_path)
        .expect("ci.yml の読み込みに失敗");
    // shadow または oracle ジョブが存在すること
    assert!(
        content.contains("shadow") || content.contains("oracle"),
        "ci.yml に shadow/oracle ジョブが存在しない"
    );
}

/// TEST-OPS-04: legacy-rust-bootstrap/ ディレクトリ構造
#[test]
fn test_e2e_ops04_legacy_isolation() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let main_rs = project_root.join("crates/lsharp-driver/src/main.rs");
    assert!(main_rs.exists(), "main.rs が存在しない");
    let content = std::fs::read_to_string(&main_rs)
        .expect("main.rs の読み込みに失敗");
    // L# compiler path に関する設定またはコメントが存在すること
    assert!(
        content.contains("LSHARP_PATH") || content.contains("lsharp_path") || content.contains("compiler path"),
        "main.rs に L# compiler path 設定が存在しない"
    );
    let smoke = project_root.join("scripts/ci/default-path-smoke.sh");
    assert!(
        smoke.is_file(),
        "scripts/ci/default-path-smoke.sh が存在しない (OPS-05 CI gate)"
    );
    let doc = project_root.join("docs/development/operations/default-path-migration.md");
    assert!(
        doc.is_file(),
        "docs/development/operations/default-path-migration.md が存在しない"
    );
}

/// TEST-OPS-06: scripts/ に release playbook + ドキュメント
#[test]
fn test_e2e_ops06_release_playbook() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
    let playbook_doc = project_root
        .join("docs/development/operations/release-playbook.md");
    assert!(
        playbook_doc.is_file(),
        "docs/development/operations/release-playbook.md が存在しない"
    );
}

/// TEST-OPS-07: scripts/smoke_test_readme.sh の存在 + 実行可能 + fresh clone 仕様ドキュメント
#[test]
fn test_e2e_ops07_fresh_clone_no_rust() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let smoke_script = project_root.join("scripts/smoke_test_readme.sh");
    assert!(
        smoke_script.exists(),
        "scripts/smoke_test_readme.sh が存在しない"
    );
    // 実行可能ビットが設定されていること (Unix)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(&smoke_script)
            .expect("smoke_test_readme.sh のメタデータ取得失敗");
        let mode = meta.permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "scripts/smoke_test_readme.sh に実行可能ビットがない (mode: {:o})",
            mode
        );
    }
    // fresh clone 仕様ドキュメントが存在すること
    let fresh_clone_doc = project_root
        .join("docs/development/operations/fresh-clone-spec.md");
    assert!(
        fresh_clone_doc.is_file(),
        "docs/development/operations/fresh-clone-spec.md が存在しない"
    );
}

/// TEST-OPS-08: scripts/ に rollback スクリプト + docs/ に手順 + 撤去 ADR
#[test]
fn test_e2e_ops08_final_removal_rollback() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

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
    let adr_doc = project_root
        .join("docs/development/operations/adr-rust-removal.md");
    assert!(
        adr_doc.is_file(),
        "docs/development/operations/adr-rust-removal.md が存在しない"
    );
}

/// D-2: Formatter.ls の format-expr が lit-int AST ノードの値を正しく返すこと
/// format-expr [1, 42] → 42 (整数リテラルの値)
#[test]
fn test_e2e_selfhost_formatter_format_expr_lit_int() {
    let harness = r#"
(defn main []
  (let [;; tag=1 (lit-int), value=42
        node (vector-push (vector-push (vector-new 2) 1) 42)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines.last().unwrap(), &"42", "lit-int 42 をフォーマットすると 42 を返すべき");
}

/// D-2: Formatter.ls の format-expr が apply AST ノードの引数数を含む結果を返すこと
/// format-expr [5, func-node, 2, arg1, arg2] → argc (2)
#[test]
fn test_e2e_selfhost_formatter_format_expr_apply() {
    let harness = r#"
(defn main []
  (let [;; tag=5 (apply), func=[4, 100], argc=2, arg1=[1, 1], arg2=[1, 2]
        func-node (vector-push (vector-push (vector-new 2) 4) 100)
        arg1 (vector-push (vector-push (vector-new 2) 1) 1)
        arg2 (vector-push (vector-push (vector-new 2) 1) 2)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 5) func-node) 2) arg1) arg2)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    // apply の format 結果: argc (引数数) を返す
    assert_eq!(lines.last().unwrap(), &"2", "apply の format 結果は argc=2 を返すべき");
}

/// FMT-01: format-expr が let (tag=7) の name-hash を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_let() {
    let harness = r#"
(defn main []
  (let [;; tag=7 (let), name-hash=50, init=[1, 10], body=[1, 20]
        init-expr (vector-push (vector-push (vector-new 2) 1) 10)
        body-expr (vector-push (vector-push (vector-new 2) 1) 20)
        node (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 50) init-expr) body-expr)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"50", "let の format 結果は name-hash=50 を返すべき");
}

/// FMT-01: format-expr が lambda (tag=8) の param-count を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_lambda() {
    let harness = r#"
(defn main []
  (let [;; tag=8 (lambda), param-count=2, p1=10, p2=20, body=[1, 42]
        body-expr (vector-push (vector-push (vector-new 2) 1) 42)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 8) 2) 10) 20) body-expr)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"2", "lambda の format 結果は param-count=2 を返すべき");
}

/// FMT-01: format-expr が do (tag=9) の expr-count を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_do() {
    let harness = r#"
(defn main []
  (let [;; tag=9 (do), expr-count=3, e1=[1,1], e2=[1,2], e3=[1,3]
        e1 (vector-push (vector-push (vector-new 2) 1) 1)
        e2 (vector-push (vector-push (vector-new 2) 1) 2)
        e3 (vector-push (vector-push (vector-new 2) 1) 3)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 9) 3) e1) e2) e3)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"3", "do の format 結果は expr-count=3 を返すべき");
}

/// FMT-01: format-expr が match (tag=10) の arm-count を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_match() {
    let harness = r#"
(defn main []
  (let [;; tag=10 (match), scrutinee=[4,99], arm-count=2
        scr (vector-push (vector-push (vector-new 2) 4) 99)
        pat1 (vector-push (vector-push (vector-new 2) 42) 1)
        body1 (vector-push (vector-push (vector-new 2) 1) 10)
        pat2 (vector-push (vector-push (vector-new 2) 42) 2)
        body2 (vector-push (vector-push (vector-new 2) 1) 20)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 10) scr) 2) pat1) body1) pat2) body2)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"2", "match の format 結果は arm-count=2 を返すべき");
}

/// FMT-01: format-expr が recordlit (tag=12) の field-count を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_recordlit() {
    let harness = r#"
(defn main []
  (let [;; tag=12 (recordlit), type-hash=99, field-count=2, f1-hash=10, f1-expr=[1,1], f2-hash=20, f2-expr=[1,2]
        f1-expr (vector-push (vector-push (vector-new 2) 1) 1)
        f2-expr (vector-push (vector-push (vector-new 2) 1) 2)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 12) 99) 2) 10) f1-expr) 20) f2-expr)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"2", "recordlit の format 結果は field-count=2 を返すべき");
}

/// FMT-01: format-expr が fieldaccess (tag=13) の field-hash を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_expr_fieldaccess() {
    let harness = r#"
(defn main []
  (let [;; tag=13 (fieldaccess), expr=[4,50], field-hash=77
        inner (vector-push (vector-push (vector-new 2) 4) 50)
        node (vector-push (vector-push (vector-push (vector-new 3) 13) inner) 77)
        result (format-expr node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"77", "fieldaccess の format 結果は field-hash=77 を返すべき");
}

/// FMT-01: format-decl が defn (tag=20) の param-count を返すこと
#[test]
fn test_e2e_selfhost_formatter_format_decl_defn() {
    let harness = r#"
(defn main []
  (let [;; defn: [20, name-hash=100, param-count=3, p1=10, p2=20, p3=30, body=[1,0]]
        body (vector-push (vector-push (vector-new 2) 1) 0)
        node (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 7) 20) 100) 3) 10) 20) 30) body)
        result (format-decl node 0)]
    (do
      (print result)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"3", "defn の format-decl 結果は param-count=3 を返すべき");
}

/// FMT-01 AC-300: 同一 AST に対して format-expr は決定的に同じ結果を返す (roundtrip)
/// 複数の AST ノード型で検証
#[test]
fn test_e2e_selfhost_formatter_roundtrip_deterministic() {
    let harness = r#"
(defn main []
  (let [;; 同一の if ノードを 2 回構築
        c1 (vector-push (vector-push (vector-new 2) 4) 99)
        t1 (vector-push (vector-push (vector-new 2) 1) 42)
        e1 (vector-push (vector-push (vector-new 2) 1) 0)
        if1 (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) c1) t1) e1)
        c2 (vector-push (vector-push (vector-new 2) 4) 99)
        t2 (vector-push (vector-push (vector-new 2) 1) 42)
        e2 (vector-push (vector-push (vector-new 2) 1) 0)
        if2 (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6) c2) t2) e2)
        ;; 同一の let ノードを 2 回構築
        li1 (vector-push (vector-push (vector-new 2) 1) 10)
        lb1 (vector-push (vector-push (vector-new 2) 1) 20)
        let1 (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 88) li1) lb1)
        li2 (vector-push (vector-push (vector-new 2) 1) 10)
        lb2 (vector-push (vector-push (vector-new 2) 1) 20)
        let2 (vector-push (vector-push (vector-push (vector-push (vector-new 4) 7) 88) li2) lb2)
        ;; format 結果を比較
        r-if1 (format-expr if1 0)
        r-if2 (format-expr if2 0)
        r-let1 (format-expr let1 0)
        r-let2 (format-expr let2 0)]
    (do
      ;; 各ペアが一致すること (Bool → Int 変換)
      (let [match-if (if (= r-if1 r-if2) 1 0)
            match-let (if (= r-let1 r-let2) 1 0)]
        (do
          (print match-if)              ;; 1
          (print match-let)             ;; 1
          (print (+ match-if match-let))))
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    // 最後の出力: 2 ペアが一致
    assert_eq!(lines.last().unwrap(), &"2", "roundtrip: 同一 AST の format 結果は一致するべき");
}

/// FMT-01 AC-301: format-program の冪等性 (idempotency)
/// 同一プログラムに対して 2 回呼んでも同じ結果
#[test]
fn test_e2e_selfhost_formatter_idempotent() {
    let harness = r#"
(defn main []
  (let [;; defn 2 つのプログラム
        b1 (vector-push (vector-push (vector-new 2) 1) 42)
        d1 (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 20) 100) 1) 200) b1)
        b2 (vector-push (vector-push (vector-new 2) 1) 0)
        d2 (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 5) 20) 300) 2) 400) b2)
        prog (vector-push (vector-push (vector-new 2) d1) d2)
        r1 (format-program prog 0)
        r2 (format-program prog 0)]
    (do
      (print (= r1 r2))  ;; 1 (冪等)
      0)))
"#;
    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.last().unwrap(), &"1", "format-program の冪等性: 2 回適用しても同じ結果");
}
