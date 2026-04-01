(module Tools.Lsp.LspServerCore)
(import Tools.Lsp.JsonRpc)

;; LspServerCore.ls - LSP サーバーのコア機能
;;
;; サーバー状態管理、メソッド定数、JSON-RPC ディスパッチ、
;; ドキュメントハンドラ (initialize/shutdown/didOpen/didChange/publishDiagnostics)、
;; パラメータアクセサ、文字分類、データ構造、JSON レンダリング (コア部)、
;; 文字列ハッシュ、診断 JSON レンダリング、JSON-RPC エンコード/パース、
;; メインループを含む。
;;
;; ナビゲーション・補完・診断ソートは LspServerNav.ls に分離。

;; === サーバー状態 ===
(defn server-state-new []
  (let [v0 (vector-new 8)
    v1 (vector-push v0 (ref-new 0)) ;; initialized フラグ
    v2 (vector-push v1 (ref-new 0)) ;; shutdown フラグ
    v3 (vector-push v2 (ref-new 0)) ;; open document 数
    v4 (vector-push v3 (ref-new 0)) ;; current uri
    v5 (vector-push v4 (ref-new "")) ;; current source
    v6 (vector-push v5 (ref-new 0)) ;; request count
    v7 (vector-push v6 (ref-new (map-new))) ;; uri -> source
    v8 (vector-push v7 (ref-new (vector-new 8)))] ;; open uri list
    v8))

(defn server-state-doc-count [state]
  (ref-get (vector-get state 2)))

(defn server-state-initialized [state]
  (ref-get (vector-get state 0)))

(defn server-state-shutdown [state]
  (ref-get (vector-get state 1)))

(defn server-state-request-count [state]
  (ref-get (vector-get state 5)))

(defn server-state-source [state]
  (ref-get (vector-get state 4)))

(defn server-state-documents [state]
  (ref-get (vector-get state 6)))

(defn server-state-uri-list [state]
  (ref-get (vector-get state 7)))

(defn server-state-uri-known-loop [uris idx count uri]
  (if (>= idx count)
    0
    (if (= (vector-get uris idx) uri)
      1
      (server-state-uri-known-loop uris (+ idx 1) count uri))))

(defn server-state-remember-uri [state uri]
  (let [uris (server-state-uri-list state)]
    (if (= (server-state-uri-known-loop uris 0 (vector-length uris) uri) 1)
      0
      (do
        (ref-set (vector-get state 7) (vector-push uris uri))
        1))))

(defn server-state-source-for-uri [state uri]
  (let [stored (map-get (server-state-documents state) uri)]
    (if (= stored 0)
      (if (= (ref-get (vector-get state 3)) uri)
        (server-state-source state)
        "")
      stored)))

(defn server-state-source-length [state]
  (string-length (server-state-source state)))

(defn server-state-set-initialized [state value]
  (ref-set (vector-get state 0) value))

(defn server-state-set-shutdown [state value]
  (ref-set (vector-get state 1) value))

(defn server-state-note-request [state]
  (ref-set (vector-get state 5) (+ (server-state-request-count state) 1)))

(defn server-state-set-document [state uri src]
  (do
    (ref-set (vector-get state 3) uri)
    (ref-set (vector-get state 4) src)
    (ref-set (vector-get state 6) (map-insert (server-state-documents state) uri src))
    (server-state-remember-uri state uri)
    0))

(defn server-state-open-document [state uri src]
  (let [current-count (server-state-doc-count state)
    known-src (server-state-source-for-uri state uri)
    next-count (if (> (string-length known-src) 0)
      current-count
      (+ current-count 1))]
    (do
      (server-state-set-document state uri src)
      (ref-set (vector-get state 2) next-count)
      (string-length src))))

(defn server-state-change-document [state uri src]
  (do
    (server-state-set-document state uri src)
    (string-length src)))

;; JsonRpc.ls と揃えた method hash
(defn lsp-method-initialize [] 1)
(defn lsp-method-shutdown [] 2)
(defn lsp-method-did-open [] 10)
(defn lsp-method-did-change [] 11)
(defn lsp-method-completion [] 20)
(defn lsp-method-hover [] 21)
(defn lsp-method-goto-def [] 22)
(defn lsp-method-formatting [] 23)
(defn lsp-method-references [] 24)
(defn lsp-method-rename [] 25)
(defn lsp-method-publish-diagnostics [] 30)

;; === LSP メソッドハンドラ (ドキュメント系) ===
;; JSON-RPC dispatch / server-loop は LspServer.ls に集約 (本ファイルは状態・ドキュメント系ハンドラ)

;; initialize: サーバー機能の宣言
;; TextDocumentSyncKind.Full を返す (AC-200)
(defn handle-initialize [params state]
  (do
    (server-state-set-initialized state 1)
    (server-state-note-request state)
    (let [capabilities (vector-new 7)]
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push
                  (vector-push capabilities 1) ;; textDocumentSync: Full
                  1) ;; hoverProvider
                1) ;; completionProvider
              1) ;; definitionProvider
            1) ;; referencesProvider
          1) ;; renameProvider
        1)))) ;; documentFormattingProvider

;; shutdown: サーバー終了準備
(defn handle-shutdown [params state]
  (do
    (server-state-set-shutdown state 1)
    (server-state-note-request state)
    0))

;; textDocument/didOpen: ドキュメントオープン通知
;; フルテキストを受け取りパースして診断を生成
(defn handle-didOpen [params state]
  (do
    (server-state-note-request state)
    (if (= (lsp-has-document-param params) 1)
      (server-state-open-document state (lsp-nav-uri params) (lsp-document-src params))
      params)))

;; textDocument/didChange: ドキュメント変更通知
;; Full sync: 全文を受け取りパースし直す (AC-201)
(defn handle-didChange [params state]
  (do
    (server-state-note-request state)
    (if (= (lsp-has-document-param params) 1)
      (server-state-change-document state (lsp-nav-uri params) (lsp-document-src params))
      params)))

;; textDocument/publishDiagnostics: 診断通知 payload を決定的 JSON に整形
;; 戻り値は [uri, diagnostics-json]
(defn handle-publish-diagnostics [params state]
  (do
    (server-state-note-request state)
    (if (> (lsp-param-count params) 1)
      (let [uri (vector-get params 0)
        diagnostics (vector-get params 1)
        diagnostics-json (render-diagnostics-json diagnostics)]
        (vector-push (vector-push (vector-new 2) uri) diagnostics-json))
      params)))

;; === textDocument 系の簡易ソース解析 ===
;; stdio なしでも hover / definition / references / completion / formatting を
;; 実ソースに近い形で返せるよう、LspServer 内部で最小限の走査を行う。

(defn lsp-param-count [params]
  (if (= params 0) 0 (vector-length params)))

(defn lsp-has-source-param [params]
  (if (> (lsp-param-count params) 3)
    (if (> (string-length (vector-get params 3)) 0) 1 0)
    0))

(defn lsp-has-document-param [params]
  (if (> (lsp-param-count params) 1) 1 0))

(defn lsp-nav-uri [params]
  (if (> (lsp-param-count params) 0) (vector-get params 0) 0))

(defn lsp-nav-line [params]
  (if (> (lsp-param-count params) 1) (vector-get params 1) 0))

(defn lsp-nav-col [params]
  (if (> (lsp-param-count params) 2) (vector-get params 2) 0))

(defn lsp-nav-src [params]
  (if (= (lsp-has-source-param params) 1) (vector-get params 3) ""))

(defn lsp-rename-new-name [params]
  (if (> (lsp-param-count params) 4) (vector-get params 4) ""))

(defn lsp-document-src [params]
  (if (= (lsp-has-document-param params) 1) (vector-get params 1) ""))

(defn lsp-session-src [params state]
  (if (= (lsp-has-source-param params) 1)
    (lsp-nav-src params)
    (server-state-source-for-uri state (lsp-nav-uri params))))

(defn lsp-session-document-src [params state]
  (if (= (lsp-has-document-param params) 1)
    (lsp-document-src params)
    (server-state-source-for-uri state (lsp-nav-uri params))))

;; === 文字分類 ===

(defn lsp-is-ws [c]
  (if (= c 32) true
    (if (= c 9) true
      (if (= c 10) true
        (= c 13)))))

(defn lsp-is-digit-char [c]
  (if (>= c 48) (<= c 57) false))

(defn lsp-is-alpha-char [c]
  (if (>= c 65)
    (if (<= c 90) true
      (if (>= c 97) (<= c 122) false))
    false))

(defn lsp-is-symbol-start [c]
  (if (lsp-is-alpha-char c) true
    (if (= c 95) true
      (if (= c 43) true
        (if (= c 45) true
          (if (= c 42) true
            (if (= c 47) true
              (if (= c 61) true
                (if (= c 60) true
                  (if (= c 62) true
                    (if (= c 33) true
                      (if (= c 63) true
                        (if (= c 38) true
                          (= c 37))))))))))))))

(defn lsp-is-symbol-char [c]
  (if (lsp-is-symbol-start c) true
    (if (lsp-is-digit-char c) true
      (if (= c 46) true
        (= c 45)))))

;; === データ構造 ===

(defn make-position [line col]
  (vector-push (vector-push (vector-new 2) line) col))

(defn position-line [pos]
  (vector-get pos 0))

(defn position-col [pos]
  (vector-get pos 1))

(defn make-range [start-line start-col end-line end-col]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) start-line)
        start-col)
      end-line)
    end-col))

(defn make-text-edit [start-line start-col end-line end-col new-text]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5) start-line)
          start-col)
        end-line)
      end-col)
    new-text))

(defn make-format-edit [start-line start-col end-line end-col new-text]
  (make-text-edit start-line start-col end-line end-col new-text))

(defn make-workspace-change [uri edits]
  (vector-push
    (vector-push (vector-new 2) uri)
    edits))

;; === JSON レンダリング (コア部) ===

(defn lsp-render-json-rpc-frame [payload]
  (render-json-rpc-frame payload))

(defn lsp-render-initialize-frame [request-id]
  (render-initialize-frame request-id))

(defn lsp-render-shutdown-frame [request-id]
  (render-shutdown-frame request-id))

(defn lsp-render-error-frame [request-id error-code error-message]
  (render-rpc-error-response-frame request-id error-code error-message))

(defn lsp-render-publish-diagnostics-frame [uri diagnostics]
  (let [uri-text (int-to-string uri)
    diagnostics-json (render-diagnostics-json diagnostics)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-text)
    payload-2 (string-concat payload-1 ",\"diagnostics\":")
    payload-3 (string-concat payload-2 diagnostics-json)
    payload (string-concat payload-3 "}}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-didopen-frame [uri source-bytes]
  (let [uri-text (int-to-string uri)
    bytes-text (int-to-string source-bytes)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-text)
    payload-2 (string-concat payload-1 ",\"sourceBytes\":")
    payload-3 (string-concat payload-2 bytes-text)
    payload (string-concat payload-3 "}}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-didchange-frame [uri source-bytes]
  (let [uri-text (int-to-string uri)
    bytes-text (int-to-string source-bytes)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-text)
    payload-2 (string-concat payload-1 ",\"sourceBytes\":")
    payload-3 (string-concat payload-2 bytes-text)
    payload (string-concat payload-3 "}}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-location-frame [request-id location]
  (render-rpc-int-vector-response-frame request-id location))

(defn lsp-render-hover-frame [request-id hover]
  (let [range (vector-get hover 0)
    contents (vector-get hover 1)
    contents-json (json-escape-string contents)
    payload-0 "{\"jsonrpc\":\"2.0\",\"id\":"
    payload-1 (string-concat payload-0 (int-to-string request-id))
    payload-2 (string-concat payload-1 ",\"result\":{\"range\":[")
    payload-3 (string-concat payload-2 (int-to-string (vector-get range 0)))
    payload-4 (string-concat payload-3 ",")
    payload-5 (string-concat payload-4 (int-to-string (vector-get range 1)))
    payload-6 (string-concat payload-5 ",")
    payload-7 (string-concat payload-6 (int-to-string (vector-get range 2)))
    payload-8 (string-concat payload-7 ",")
    payload-9 (string-concat payload-8 (int-to-string (vector-get range 3)))
    payload-10 (string-concat payload-9 "],\"contents\":\"")
    payload-11 (string-concat payload-10 contents-json)
    payload (string-concat payload-11 "\"}}")]
    (render-json-rpc-frame payload)))

;; === 文字列ハッシュ・シンボル情報 ===

(defn lsp-parse-content-length [header-value]
  (parse-content-length header-value))

(defn lsp-string-hash-loop [src pos end acc]
  (if (>= pos end)
    acc
    (lsp-string-hash-loop src (+ pos 1) end (+ (string-char-at src pos) (* acc 31)))))

(defn lsp-string-hash [src]
  (lsp-string-hash-loop src 0 (string-length src) 0))

(defn lsp-substring-hash [src start end]
  (lsp-string-hash-loop src start end 0))

(defn make-symbol-info [start end]
  (vector-push
    (vector-push (vector-new 2) start)
    end))

(defn empty-symbol-info []
  (make-symbol-info (- 0 1) (- 0 1)))

(defn symbol-info-start [info]
  (vector-get info 0))

(defn symbol-info-end [info]
  (vector-get info 1))

;; === 診断 JSON text renderer ===

;; 現在の diagnostics shape は [severity, rule-id, line, col, msg-hash, source]
;; なので int-only の deterministic JSON text に落とし込む。
(defn render-diagnostic-json [diag]
  (let [severity (vector-get diag 0)
    rule-id (vector-get diag 1)
    line (vector-get diag 2)
    col (vector-get diag 3)
    message-hash (vector-get diag 4)
    source (vector-get diag 5)
    source-text (int-to-string source)
    severity-text (int-to-string severity)
    rule-text (int-to-string rule-id)
    line-text (int-to-string line)
    col-text (int-to-string col)
    message-text (int-to-string message-hash)
    out0 "{\"source\":"
    out1 (string-concat out0 source-text)
    out2 (string-concat out1 ",\"severity\":")
    out3 (string-concat out2 severity-text)
    out4 (string-concat out3 ",\"rule\":")
    out5 (string-concat out4 rule-text)
    out6 (string-concat out5 ",\"line\":")
    out7 (string-concat out6 line-text)
    out8 (string-concat out7 ",\"col\":")
    out9 (string-concat out8 col-text)
    out10 (string-concat out9 ",\"messageHash\":")
    out11 (string-concat out10 message-text)]
    (string-concat out11 "}")))

(defn render-diagnostics-json-loop [diags idx len out]
  (if (>= idx len)
    out
    (let [elem-text (render-diagnostic-json (vector-get diags idx))
      next-out (if (= idx 0)
        (string-concat out elem-text)
        (string-concat out (string-concat "," elem-text)))]
      (render-diagnostics-json-loop diags (+ idx 1) len next-out))))

(defn render-diagnostics-json [diags]
  (string-concat
    "["
    (string-concat
      (render-diagnostics-json-loop diags 0 (vector-length diags) "")
      "]")))

;; === JSON-RPC エンコード/パース ===

;; encode-json-rpc-response: JSON-RPC 2.0 レスポンス構造を生成
;; [jsonrpc-version(=2), id, result]
(defn encode-json-rpc-response [id result]
  (vector-push (vector-push (vector-push (vector-new 3) 2) id) result))

(defn render-json-rpc-error-response [request-id error-code error-message]
  (let [message-json (json-escape-string error-message)]
    (string-concat
      "{\"jsonrpc\":\"2.0\",\"id\":"
      (string-concat
        (int-to-string request-id)
        (string-concat
          ",\"error\":{"
          (string-concat
            "\"code\":"
            (string-concat
              (int-to-string error-code)
              (string-concat
                ",\"message\":\""
                (string-concat message-json "\"}}")))))))))

;; parse-json-rpc-request: JSON-RPC リクエストから method + params を抽出
;; 入力: [jsonrpc-version, id, method-id, params]
;; 出力: [method-id, params]
(defn parse-json-rpc-request [msg]
  (let [method-id (vector-get msg 2)
    params (vector-get msg 3)]
    (vector-push (vector-push (vector-new 2) method-id) params)))

