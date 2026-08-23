(module Tools.Lsp.LspServerCore)
(import Tools.Lsp.JsonRpc)

(defn push-int-vector-local [dst value]
  (do
    (root_push dst)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        next-dst))))

(defn push-object-vector-local [dst value]
  (do
    (root_push dst)
    (root_push value)
    (let [next-dst (vector-push dst value)]
      (do
        (root_pop)
        (root_pop)
        next-dst))))

(defn ref-map-get-safe [map-ref key]
  (let [map-value (ref-get map-ref)]
    (do
      (root_push map-value)
      (let [value (map-get map-value key)]
        (do
          (root_pop)
          value)))))

(defn ref-map-insert-object-safe [map-ref key value]
  (let [map-value (ref-get map-ref)]
    (do
      (root_push map-value)
      (root_push value)
      (let [next-map (map-insert map-value key value)]
        (do
          (root_pop)
          (root_pop)
          next-map)))))

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
  (let [v0 (vector-new 10)
    v1 (push-object-vector-local v0 (ref-new 0)) ;; initialized フラグ
    v2 (push-object-vector-local v1 (ref-new 0)) ;; shutdown フラグ
    v3 (push-object-vector-local v2 (ref-new 0)) ;; open document 数
    v4 (push-object-vector-local v3 (ref-new 0)) ;; current uri
    v5 (push-object-vector-local v4 (ref-new "")) ;; current source
    v6 (push-object-vector-local v5 (ref-new 0)) ;; request count
    v7 (push-object-vector-local v6 (ref-new (map-new))) ;; uri -> source
    v8 (push-object-vector-local v7 (ref-new (vector-new 8))) ;; open uri list
    v9 (push-object-vector-local v8 (ref-new "")) ;; current path
    v10 (push-object-vector-local v9 (ref-new (map-new))) ;; uri -> path
    v11 (push-object-vector-local v10 (ref-new (map-new)))] ;; uri -> wire URI
    v11))

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

(defn server-state-documents-ref [state]
  (vector-get state 6))

(defn server-state-uri-list-ref [state]
  (vector-get state 7))

(defn server-state-path [state]
  (ref-get (vector-get state 8)))

(defn server-state-document-paths [state]
  (ref-get (vector-get state 9)))

(defn server-state-document-paths-ref [state]
  (vector-get state 9))

(defn server-state-uri-texts [state]
  (ref-get (vector-get state 10)))

(defn server-state-uri-texts-ref [state]
  (vector-get state 10))

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
        (ref-set (server-state-uri-list-ref state) (push-int-vector-local uris uri))
        1))))

(defn server-state-source-for-uri [state uri]
  (let [stored (ref-map-get-safe (server-state-documents-ref state) uri)]
    (if (= stored 0)
      (if (= (ref-get (vector-get state 3)) uri)
        (server-state-source state)
        "")
      stored)))

(defn server-state-path-for-uri [state uri]
  (let [stored (ref-map-get-safe (server-state-document-paths-ref state) uri)]
    (if (= stored 0)
      (if (= (ref-get (vector-get state 3)) uri)
        (server-state-path state)
        "")
      stored)))

(defn server-state-uri-text-for-uri [state uri]
  (let [stored (ref-map-get-safe (server-state-uri-texts-ref state) uri)]
    (if (= stored 0) "" stored)))

(defn server-state-source-length [state]
  (string-length (server-state-source state)))

(defn server-state-set-initialized [state value]
  (ref-set (vector-get state 0) value))

(defn server-state-set-shutdown [state value]
  (ref-set (vector-get state 1) value))

(defn server-state-note-request [state]
  (ref-set (vector-get state 5) (+ (server-state-request-count state) 1)))

(defn server-state-set-document-with-path-and-uri [state uri src path uri-text]
  (let [effective-path (if (> (string-length path) 0) path (server-state-path-for-uri state uri))
    effective-uri-text (if (> (string-length uri-text) 0) uri-text (server-state-uri-text-for-uri state uri))
    next-paths (if (> (string-length effective-path) 0)
      (ref-map-insert-object-safe (server-state-document-paths-ref state) uri effective-path)
      (server-state-document-paths state))
    next-uri-texts (if (> (string-length effective-uri-text) 0)
      (ref-map-insert-object-safe (server-state-uri-texts-ref state) uri effective-uri-text)
      (server-state-uri-texts state))]
    (do
      (ref-set (vector-get state 3) uri)
      (ref-set (vector-get state 4) src)
      (ref-set (server-state-documents-ref state) (ref-map-insert-object-safe (server-state-documents-ref state) uri src))
      (ref-set (vector-get state 8) effective-path)
      (ref-set (server-state-document-paths-ref state) next-paths)
      (ref-set (server-state-uri-texts-ref state) next-uri-texts)
      (server-state-remember-uri state uri)
      0)))

(defn server-state-set-document-with-path [state uri src path]
  (server-state-set-document-with-path-and-uri state uri src path ""))

(defn server-state-set-document [state uri src]
  (server-state-set-document-with-path state uri src ""))

(defn server-state-open-document [state uri src]
  (server-state-open-document-with-path state uri src ""))

(defn server-state-open-document-with-path [state uri src path]
  (server-state-open-document-with-path-and-uri state uri src path ""))

(defn server-state-open-document-with-path-and-uri [state uri src path uri-text]
  (let [current-count (server-state-doc-count state)
    known-src (server-state-source-for-uri state uri)
    next-count (if (> (string-length known-src) 0)
      current-count
      (+ current-count 1))]
    (do
      (server-state-set-document-with-path-and-uri state uri src path uri-text)
      (ref-set (vector-get state 2) next-count)
      (string-length src))))

(defn server-state-change-document [state uri src]
  (server-state-change-document-with-path state uri src ""))

(defn server-state-change-document-with-path [state uri src path]
  (server-state-change-document-with-path-and-uri state uri src path ""))

(defn server-state-change-document-with-path-and-uri [state uri src path uri-text]
  (do
    (server-state-set-document-with-path-and-uri state uri src path uri-text)
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
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local
              (push-int-vector-local
                (push-int-vector-local
                  (push-int-vector-local capabilities 1) ;; textDocumentSync: Full
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
      (server-state-open-document-with-path-and-uri
        state
        (lsp-nav-uri params)
        (lsp-document-src params)
        (lsp-document-path params)
        (lsp-document-uri-text params))
      params)))

;; textDocument/didChange: ドキュメント変更通知
;; Full sync: 全文を受け取りパースし直す (AC-201)
(defn handle-didChange [params state]
  (do
    (server-state-note-request state)
    (if (= (lsp-has-document-param params) 1)
      (server-state-change-document-with-path-and-uri
        state
        (lsp-nav-uri params)
        (lsp-document-src params)
        (lsp-document-path params)
        (lsp-document-uri-text params))
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
        (push-object-vector-local (push-int-vector-local (vector-new 2) uri) diagnostics-json))
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

(defn lsp-document-path [params]
  (if (> (lsp-param-count params) 2) (vector-get params 2) ""))

(defn lsp-document-uri-text [params]
  (if (> (lsp-param-count params) 3) (vector-get params 3) ""))

(defn lsp-session-src [params state]
  (if (= (lsp-has-source-param params) 1)
    (lsp-nav-src params)
    (server-state-source-for-uri state (lsp-nav-uri params))))

;; inline の source slot が空なら open document state へ落ちる。
;; slot の有無だけで判定すると、source を持たない request が空文字列を
;; source として受け取り、didOpen 済みの内容を無視する (I-56)
(defn lsp-session-document-src [params state]
  (let [inline (lsp-document-src params)]
    (if (> (string-length inline) 0)
      inline
      (server-state-source-for-uri state (lsp-nav-uri params)))))

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
  (push-int-vector-local (push-int-vector-local (vector-new 2) line) col))

(defn position-line [pos]
  (vector-get pos 0))

(defn position-col [pos]
  (vector-get pos 1))

(defn make-range [start-line start-col end-line end-col]
  (push-int-vector-local
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local (vector-new 4) start-line)
        start-col)
      end-line)
    end-col))

(defn make-text-edit [start-line start-col end-line end-col new-text]
  (push-object-vector-local
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local (vector-new 5) start-line)
          start-col)
        end-line)
      end-col)
    new-text))

(defn make-format-edit [start-line start-col end-line end-col new-text]
  (make-text-edit start-line start-col end-line end-col new-text))

(defn make-workspace-change [uri edits]
  (push-object-vector-local
    (push-int-vector-local (vector-new 2) uri)
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

(defn lsp-render-uri-value-json [state uri]
  (let [uri-text (server-state-uri-text-for-uri state uri)]
    (if (> (string-length uri-text) 0)
      (string-concat "\"" (string-concat (json-escape-string uri-text) "\""))
      (int-to-string uri))))

(defn lsp-render-publish-diagnostics-frame [uri diagnostics]
  (let [uri-text (int-to-string uri)
    diagnostics-json (render-diagnostics-json diagnostics)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-text)
    payload-2 (string-concat payload-1 ",\"diagnostics\":")
    payload-3 (string-concat payload-2 diagnostics-json)
    payload (string-concat payload-3 "}}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-publish-diagnostics-frame-with-state [state uri diagnostics]
  (let [uri-json (lsp-render-uri-value-json state uri)
    diagnostics-json (render-diagnostics-json diagnostics)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-json)
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

(defn lsp-render-didopen-frame-with-state [state uri source-bytes]
  (let [uri-json (lsp-render-uri-value-json state uri)
    bytes-text (int-to-string source-bytes)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-json)
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

(defn lsp-render-didchange-frame-with-state [state uri source-bytes]
  (let [uri-json (lsp-render-uri-value-json state uri)
    bytes-text (int-to-string source-bytes)
    payload-0 "{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\"params\":{\"uri\":"
    payload-1 (string-concat payload-0 uri-json)
    payload-2 (string-concat payload-1 ",\"sourceBytes\":")
    payload-3 (string-concat payload-2 bytes-text)
    payload (string-concat payload-3 "}}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-range-json [range]
  (let [payload-0 "{\"start\":{\"line\":"
    payload-1 (string-concat payload-0 (int-to-string (vector-get range 0)))
    payload-2 (string-concat payload-1 ",\"character\":")
    payload-3 (string-concat payload-2 (int-to-string (vector-get range 1)))
    payload-4 (string-concat payload-3 "},\"end\":{\"line\":")
    payload-5 (string-concat payload-4 (int-to-string (vector-get range 2)))
    payload-6 (string-concat payload-5 ",\"character\":")
    payload-7 (string-concat payload-6 (int-to-string (vector-get range 3)))
    payload (string-concat payload-7 "}}")]
    payload))

;; LSP wire の Position は zero-based。内部解析位置 (1-based) から境界で変換する。
(defn lsp-render-wire-range-json [range]
  (let [payload-0 "{\"start\":{\"line\":"
    payload-1 (string-concat payload-0 (int-to-string (- (vector-get range 0) 1)))
    payload-2 (string-concat payload-1 ",\"character\":")
    payload-3 (string-concat payload-2 (int-to-string (- (vector-get range 1) 1)))
    payload-4 (string-concat payload-3 "},\"end\":{\"line\":")
    payload-5 (string-concat payload-4 (int-to-string (- (vector-get range 2) 1)))
    payload-6 (string-concat payload-5 ",\"character\":")
    payload-7 (string-concat payload-6 (int-to-string (- (vector-get range 3) 1)))
    payload (string-concat payload-7 "}}")]
    payload))

(defn lsp-render-hover-frame [request-id hover]
  (let [range (vector-get hover 0)
    contents (vector-get hover 1)
    contents-json (json-escape-string contents)
    payload-0 "{\"jsonrpc\":\"2.0\",\"id\":"
    payload-1 (string-concat payload-0 (int-to-string request-id))
    payload-2 (string-concat payload-1 ",\"result\":{\"range\":")
    payload-3 (string-concat payload-2 (lsp-render-wire-range-json range))
    payload-4 (string-concat payload-3 ",\"contents\":\"")
    payload-5 (string-concat payload-4 contents-json)
    payload (string-concat payload-5 "\"}}")]
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

(defn lsp-uri-key-from-text [uri-text]
  (let [hash (lsp-string-hash uri-text)]
    (if (< hash 0)
      (- 0 hash)
      (if (= hash 0) 2 hash))))

(defn lsp-substring-hash [src start end]
  (lsp-string-hash-loop src start end 0))

(defn make-symbol-info [start end]
  (push-int-vector-local
    (push-int-vector-local (vector-new 2) start)
    end))

(defn empty-symbol-info []
  (make-symbol-info (- 0 1) (- 0 1)))

(defn symbol-info-start [info]
  (vector-get info 0))

(defn symbol-info-end [info]
  (vector-get info 1))

;; === 診断 JSON text renderer ===

;; 現在の diagnostics shape は [severity, rule-id, line, col, msg-hash, source]
;; なので legacy 経路は int-only の deterministic JSON text に落とし込む。
(defn render-legacy-diagnostic-json [diag]
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

;; enriched parse diagnostic は
;; [severity, rule-id, line, col, msg-hash, source, end-line, end-col, code-text, message-text]
;; を持ち、先頭6要素を legacy sort/dedup と共有する。
(defn render-standard-diagnostic-json [diag]
  (let [severity (vector-get diag 0)
    start-line (- (vector-get diag 2) 1)
    start-col (- (vector-get diag 3) 1)
    end-line (- (vector-get diag 6) 1)
    end-col (- (vector-get diag 7) 1)
    code-text (json-escape-string (vector-get diag 8))
    message-text (json-escape-string (vector-get diag 9))
    out0 "{\"range\":{\"start\":{\"line\":"
    out1 (string-concat out0 (int-to-string start-line))
    out2 (string-concat out1 ",\"character\":")
    out3 (string-concat out2 (int-to-string start-col))
    out4 (string-concat out3 "},\"end\":{\"line\":")
    out5 (string-concat out4 (int-to-string end-line))
    out6 (string-concat out5 ",\"character\":")
    out7 (string-concat out6 (int-to-string end-col))
    out8 (string-concat out7 "}},\"severity\":")
    out9 (string-concat out8 (int-to-string severity))
    out10 (string-concat out9 ",\"code\":\"")
    out11 (string-concat out10 code-text)
    out12 (string-concat out11 "\",\"source\":\"lsharp\",\"message\":\"")
    out13 (string-concat out12 message-text)]
    (string-concat out13 "\"}")))

(defn render-diagnostic-json [diag]
  (if (>= (vector-length diag) 10)
    (render-standard-diagnostic-json diag)
    (render-legacy-diagnostic-json diag)))

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
  (push-object-vector-local (push-int-vector-local (push-int-vector-local (vector-new 3) 2) id) result))

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
    (push-object-vector-local (push-int-vector-local (vector-new 2) method-id) params)))
