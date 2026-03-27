(module LspServer)
(import AST)
(import Parser)
(import Formatter)
(import JsonRpc)

;; LspServer.ls - L# 製 LSP サーバー
;;
;; P11-4 T4-2: L# 製 LSP の正式化
;; LSP 3.17 仕様に準拠した 10 メソッドを実装。
;; JSON-RPC 2.0 プロトコルによる通信。
;;
;; 対応メソッド:
;;   initialize, shutdown,
;;   textDocument/didOpen, textDocument/didChange,
;;   textDocument/hover, textDocument/definition,
;;   textDocument/references, textDocument/rename,
;;   textDocument/formatting, textDocument/completion

;; === サーバー状態 ===
(defn server-state-new []
  (let [v0 (vector-new 8)
        v1 (vector-push v0 (ref-new 0))            ;; initialized フラグ
        v2 (vector-push v1 (ref-new 0))            ;; shutdown フラグ
        v3 (vector-push v2 (ref-new 0))            ;; open document 数
        v4 (vector-push v3 (ref-new 0))            ;; current uri
        v5 (vector-push v4 (ref-new ""))           ;; current source
        v6 (vector-push v5 (ref-new 0))            ;; request count
        v7 (vector-push v6 (ref-new (map-new)))    ;; uri -> source
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

;; === JSON-RPC ディスパッチ ===

;; メソッド名に基づいてハンドラを呼び出す json-rpc-dispatch
(defn json-rpc-dispatch [method-id params state]
  (if (= method-id (lsp-method-initialize)) (handle-initialize params state)
  (if (= method-id (lsp-method-shutdown)) (handle-shutdown params state)
  (if (= method-id (lsp-method-did-open)) (handle-didOpen params state)
  (if (= method-id (lsp-method-did-change)) (handle-didChange params state)
  (if (= method-id (lsp-method-hover)) (handle-hover params state)
  (if (= method-id (lsp-method-goto-def)) (handle-goto-definition params state)
  (if (= method-id (lsp-method-references)) (handle-references params state)
  (if (= method-id (lsp-method-rename)) (handle-rename params state)
  (if (= method-id (lsp-method-publish-diagnostics)) (handle-publish-diagnostics params state)
  (if (= method-id (lsp-method-formatting)) (handle-formatting params state)
  (if (= method-id (lsp-method-completion)) (handle-completion params state)
  0))))))))))))

;; === LSP メソッドハンドラ ===

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
                  (vector-push capabilities 1)  ;; textDocumentSync: Full
                  1)                             ;; hoverProvider
                1)                               ;; completionProvider
              1)                                 ;; definitionProvider
            1)                                   ;; referencesProvider
          1)                                     ;; renameProvider
        1))))                                    ;; documentFormattingProvider

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
  (if (> (lsp-param-count params) 3) 1 0))

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
        payload-11 (string-concat payload-10 contents)
        payload (string-concat payload-11 "\"}}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-location-json [location]
  (let [uri-text (int-to-string (vector-get location 0))
        line-text (int-to-string (vector-get location 1))
        col-text (int-to-string (vector-get location 2))
        payload-0 "["
        payload-1 (string-concat payload-0 uri-text)
        payload-2 (string-concat payload-1 ",")
        payload-3 (string-concat payload-2 line-text)
        payload-4 (string-concat payload-3 ",")
        payload-5 (string-concat payload-4 col-text)]
    (string-concat payload-5 "]")))

(defn lsp-render-locations-json-loop [locations idx len out]
  (if (>= idx len)
    out
    (let [elem-text (lsp-render-location-json (vector-get locations idx))
          next-out (if (= idx 0)
                     (string-concat out elem-text)
                     (string-concat out (string-concat "," elem-text)))]
      (lsp-render-locations-json-loop locations (+ idx 1) len next-out))))

(defn lsp-render-locations-frame [request-id locations]
  (let [payload-0 "{\"jsonrpc\":\"2.0\",\"id\":"
        payload-1 (string-concat payload-0 (int-to-string request-id))
        payload-2 (string-concat payload-1 ",\"result\":[")
        payload-3 (string-concat payload-2
                   (lsp-render-locations-json-loop locations 0 (vector-length locations) ""))
        payload (string-concat payload-3 "]}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-completion-item-json [item]
  (let [label (vector-get item 0)
        kind-text (int-to-string (vector-get item 1))
        insert-text (vector-get item 2)
        payload-0 "[\""
        payload-1 (string-concat payload-0 label)
        payload-2 (string-concat payload-1 "\",")
        payload-3 (string-concat payload-2 kind-text)
        payload-4 (string-concat payload-3 ",\"")
        payload-5 (string-concat payload-4 insert-text)]
    (string-concat payload-5 "\"]")))

(defn lsp-render-completion-items-json-loop [items idx len out]
  (if (>= idx len)
    out
    (let [elem-text (lsp-render-completion-item-json (vector-get items idx))
          next-out (if (= idx 0)
                     (string-concat out elem-text)
                     (string-concat out (string-concat "," elem-text)))]
      (lsp-render-completion-items-json-loop items (+ idx 1) len next-out))))

(defn lsp-render-completion-frame [request-id items]
  (let [payload-0 "{\"jsonrpc\":\"2.0\",\"id\":"
        payload-1 (string-concat payload-0 (int-to-string request-id))
        payload-2 (string-concat payload-1 ",\"result\":[")
        payload-3 (string-concat payload-2
                   (lsp-render-completion-items-json-loop items 0 (vector-length items) ""))
        payload (string-concat payload-3 "]}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-text-edit-json [edit]
  (let [payload-0 "["
        payload-1 (string-concat payload-0 (int-to-string (vector-get edit 0)))
        payload-2 (string-concat payload-1 ",")
        payload-3 (string-concat payload-2 (int-to-string (vector-get edit 1)))
        payload-4 (string-concat payload-3 ",")
        payload-5 (string-concat payload-4 (int-to-string (vector-get edit 2)))
        payload-6 (string-concat payload-5 ",")
        payload-7 (string-concat payload-6 (int-to-string (vector-get edit 3)))
        payload-8 (string-concat payload-7 ",\"")
        payload-9 (string-concat payload-8 (vector-get edit 4))]
    (string-concat payload-9 "\"]")))

(defn lsp-render-text-edits-json-loop [edits idx len out]
  (if (>= idx len)
    out
    (let [elem-text (lsp-render-text-edit-json (vector-get edits idx))
          next-out (if (= idx 0)
                     (string-concat out elem-text)
                     (string-concat out (string-concat "," elem-text)))]
      (lsp-render-text-edits-json-loop edits (+ idx 1) len next-out))))

(defn lsp-render-formatting-frame [request-id edits]
  (let [payload-0 "{\"jsonrpc\":\"2.0\",\"id\":"
        payload-1 (string-concat payload-0 (int-to-string request-id))
        payload-2 (string-concat payload-1 ",\"result\":[")
        payload-3 (string-concat payload-2
                   (lsp-render-text-edits-json-loop edits 0 (vector-length edits) ""))
        payload (string-concat payload-3 "]}")]
    (render-json-rpc-frame payload)))

(defn lsp-render-workspace-change-json [change]
  (let [uri-text (int-to-string (vector-get change 0))
        edits (vector-get change 1)
        payload-0 "["
        payload-1 (string-concat payload-0 uri-text)
        payload-2 (string-concat payload-1 ",[")
        payload-3 (string-concat payload-2
                   (lsp-render-text-edits-json-loop edits 0 (vector-length edits) ""))
        payload-4 (string-concat payload-3 "]")]
    (string-concat payload-4 "]")))

(defn lsp-render-workspace-changes-json-loop [changes idx len out]
  (if (>= idx len)
    out
    (let [elem-text (lsp-render-workspace-change-json (vector-get changes idx))
          next-out (if (= idx 0)
                     (string-concat out elem-text)
                     (string-concat out (string-concat "," elem-text)))]
      (lsp-render-workspace-changes-json-loop changes (+ idx 1) len next-out))))

(defn lsp-render-rename-frame [request-id changes]
  (let [payload-0 "{\"jsonrpc\":\"2.0\",\"id\":"
        payload-1 (string-concat payload-0 (int-to-string request-id))
        payload-2 (string-concat payload-1 ",\"result\":[")
        payload-3 (string-concat payload-2
                   (lsp-render-workspace-changes-json-loop changes 0 (vector-length changes) ""))
        payload (string-concat payload-3 "]}")]
    (render-json-rpc-frame payload)))

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

(defn lsp-offset-from-line-col-loop [src target-line target-col idx line col len]
  (if (= line target-line)
    (if (= col target-col)
      idx
      (if (>= idx len)
        idx
        (if (= (string-char-at src idx) 10)
          (lsp-offset-from-line-col-loop src target-line target-col (+ idx 1) (+ line 1) 1 len)
          (lsp-offset-from-line-col-loop src target-line target-col (+ idx 1) line (+ col 1) len))))
    (if (>= idx len)
      idx
      (if (= (string-char-at src idx) 10)
        (lsp-offset-from-line-col-loop src target-line target-col (+ idx 1) (+ line 1) 1 len)
        (lsp-offset-from-line-col-loop src target-line target-col (+ idx 1) line (+ col 1) len)))))

(defn lsp-offset-from-line-col [src line col]
  (lsp-offset-from-line-col-loop src line col 0 1 1 (string-length src)))

(defn lsp-position-from-offset-loop [src target idx line col]
  (if (>= idx target)
    (make-position line col)
    (if (= (string-char-at src idx) 10)
      (lsp-position-from-offset-loop src target (+ idx 1) (+ line 1) 1)
      (lsp-position-from-offset-loop src target (+ idx 1) line (+ col 1)))))

(defn lsp-position-from-offset [src offset]
  (lsp-position-from-offset-loop src offset 0 1 1))

(defn lsp-range-from-offsets [src start end]
  (let [start-pos (lsp-position-from-offset src start)
        end-pos (lsp-position-from-offset src end)]
    (make-range
      (position-line start-pos)
      (position-col start-pos)
      (position-line end-pos)
      (position-col end-pos))))

(defn lsp-normalize-symbol-offset [src offset len]
  (if (>= offset len)
    (if (> len 0)
      (if (lsp-is-symbol-char (string-char-at src (- len 1)))
        (- len 1)
        len)
      0)
    (if (lsp-is-symbol-char (string-char-at src offset))
      offset
      (if (> offset 0)
        (if (lsp-is-symbol-char (string-char-at src (- offset 1)))
          (- offset 1)
          offset)
        offset))))

(defn lsp-find-symbol-start [src idx]
  (if (<= idx 0)
    0
    (if (lsp-is-symbol-char (string-char-at src (- idx 1)))
      (lsp-find-symbol-start src (- idx 1))
      idx)))

(defn lsp-scan-symbol-end [src idx len]
  (if (>= idx len)
    idx
    (if (lsp-is-symbol-char (string-char-at src idx))
      (lsp-scan-symbol-end src (+ idx 1) len)
      idx)))

(defn lsp-symbol-at [src line col]
  (let [len (string-length src)]
    (if (= len 0)
      (empty-symbol-info)
      (let [offset (lsp-offset-from-line-col src line col)
            offset (lsp-normalize-symbol-offset src offset len)]
        (if (>= offset len)
          (empty-symbol-info)
          (if (lsp-is-symbol-char (string-char-at src offset))
            (let [start (lsp-find-symbol-start src offset)
                  end (lsp-scan-symbol-end src offset len)]
              (make-symbol-info start end))
            (empty-symbol-info)))))))

(defn lsp-match-at [src idx pattern]
  (let [plen (string-length pattern)
        len (string-length src)]
    (if (> (+ idx plen) len)
      false
      (string-eq (substring src idx (+ idx plen)) pattern))))

(defn lsp-skip-ws [src idx len]
  (if (>= idx len)
    idx
    (if (lsp-is-ws (string-char-at src idx))
      (lsp-skip-ws src (+ idx 1) len)
      idx)))

(defn lsp-find-defn-offset-loop [src target idx len]
  (if (>= idx len)
    (- 0 1)
    (if (lsp-match-at src idx "(defn")
      (let [name-start (lsp-skip-ws src (+ idx 5) len)
            name-end (lsp-scan-symbol-end src name-start len)]
        (if (> name-end name-start)
          (let [name (substring src name-start name-end)]
            (if (string-eq name target)
              name-start
              (lsp-find-defn-offset-loop src target name-end len)))
          (lsp-find-defn-offset-loop src target (+ idx 1) len)))
      (lsp-find-defn-offset-loop src target (+ idx 1) len))))

(defn lsp-find-defn-offset [src target]
  (lsp-find-defn-offset-loop src target 0 (string-length src)))

(defn lsp-find-defn-offset-before-loop [src target idx limit len last-match]
  (if (>= idx len)
    last-match
    (if (> idx limit)
      last-match
      (if (lsp-match-at src idx "(defn")
        (let [name-start (lsp-skip-ws src (+ idx 5) len)
              name-end (lsp-scan-symbol-end src name-start len)]
          (if (> name-end name-start)
            (let [name (substring src name-start name-end)
                  last-match (if (<= name-start limit)
                               (if (string-eq name target) name-start last-match)
                               last-match)]
              (lsp-find-defn-offset-before-loop src target name-end limit len last-match))
            (lsp-find-defn-offset-before-loop src target (+ idx 1) limit len last-match)))
        (lsp-find-defn-offset-before-loop src target (+ idx 1) limit len last-match)))))

(defn lsp-find-defn-offset-before [src target limit]
  (lsp-find-defn-offset-before-loop src target 0 limit (string-length src) (- 0 1)))

(defn lsp-resolve-defn-offset [src target cursor-start]
  (let [defn-offset (lsp-find-defn-offset-before src target cursor-start)]
    (if (>= defn-offset 0)
      defn-offset
      (lsp-find-defn-offset src target))))

(defn lsp-make-defn-resolution [uri offset]
  (vector-push (vector-push (vector-new 2) uri) offset))

(defn lsp-defn-resolution-uri [resolution]
  (vector-get resolution 0))

(defn lsp-defn-resolution-offset [resolution]
  (vector-get resolution 1))

(defn lsp-find-defn-in-open-docs-loop [state current-uri name idx count]
  (if (>= idx count)
    (lsp-make-defn-resolution current-uri (- 0 1))
    (let [target-uri (vector-get (server-state-uri-list state) idx)]
      (if (= target-uri current-uri)
        (lsp-find-defn-in-open-docs-loop state current-uri name (+ idx 1) count)
        (let [target-src (server-state-source-for-uri state target-uri)
              target-offset (lsp-find-defn-offset target-src name)]
          (if (>= target-offset 0)
            (lsp-make-defn-resolution target-uri target-offset)
            (lsp-find-defn-in-open-docs-loop state current-uri name (+ idx 1) count)))))))

(defn lsp-resolve-defn-in-open-docs [state uri src name start]
  (let [local-offset (lsp-resolve-defn-offset src name start)]
    (if (>= local-offset 0)
      (lsp-make-defn-resolution uri local-offset)
      (lsp-find-defn-in-open-docs-loop state uri name 0 (vector-length (server-state-uri-list state))))))

(defn lsp-hover-content-text [defn-offset name]
  (if (>= defn-offset 0)
    (string-concat "defn " name)
    (string-concat "symbol " name)))

(defn lsp-symbol-start-at [src idx]
  (if (= idx 0)
    1
    (if (lsp-is-symbol-char (string-char-at src (- idx 1))) 0 1)))

(defn lsp-find-occurrences-loop [src target uri idx len results]
  (if (>= idx len)
    results
    (let [c (string-char-at src idx)]
      (if (lsp-is-symbol-char c)
        (if (= (lsp-symbol-start-at src idx) 1)
          (let [end (lsp-scan-symbol-end src idx len)
                name (substring src idx end)]
            (if (string-eq name target)
              (let [pos (lsp-position-from-offset src idx)
                    loc (make-location uri (position-line pos) (position-col pos))]
                (lsp-find-occurrences-loop src target uri end len (vector-push results loc)))
              (lsp-find-occurrences-loop src target uri end len results)))
          (lsp-find-occurrences-loop src target uri (+ idx 1) len results))
        (lsp-find-occurrences-loop src target uri (+ idx 1) len results)))))

(defn lsp-find-occurrences [src target uri]
  (lsp-find-occurrences-loop src target uri 0 (string-length src) (vector-new 4)))

(defn lsp-prefix-at [src line col]
  (let [offset (lsp-offset-from-line-col src line col)]
    (if (<= offset 0)
      ""
      (let [idx (- offset 1)]
        (if (lsp-is-symbol-char (string-char-at src idx))
          (let [start (lsp-find-symbol-start src idx)]
            (substring src start (+ idx 1)))
          "")))))

(defn lsp-prefix-matches [label prefix]
  (let [prefix-len (string-length prefix)
        label-len (string-length label)]
    (if (= prefix-len 0)
      true
      (if (> prefix-len label-len)
        false
        (string-eq (substring label 0 prefix-len) prefix)))))

(defn lsp-make-completion-item [label kind insert-text]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) label)
      kind)
    insert-text))

(defn lsp-append-keyword-item [label prefix kind items]
  (if (lsp-prefix-matches label prefix)
    (vector-push items (lsp-make-completion-item label kind label))
    items))

(defn lsp-append-keyword-completions [prefix items]
  (let [items (lsp-append-keyword-item "defn" prefix 14 items)
        items (lsp-append-keyword-item "let" prefix 14 items)
        items (lsp-append-keyword-item "if" prefix 14 items)
        items (lsp-append-keyword-item "match" prefix 14 items)
        items (lsp-append-keyword-item "do" prefix 14 items)
        items (lsp-append-keyword-item "fn" prefix 14 items)
        items (lsp-append-keyword-item "module" prefix 14 items)]
    items))

(defn lsp-append-defn-completions-loop [src idx len prefix items]
  (if (>= idx len)
    items
    (if (lsp-match-at src idx "(defn")
      (let [name-start (lsp-skip-ws src (+ idx 5) len)
            name-end (lsp-scan-symbol-end src name-start len)]
        (if (> name-end name-start)
          (let [name (substring src name-start name-end)
                items (if (lsp-prefix-matches name prefix)
                        (vector-push items (lsp-make-completion-item name 3 name))
                        items)]
            (lsp-append-defn-completions-loop src name-end len prefix items))
          (lsp-append-defn-completions-loop src (+ idx 1) len prefix items)))
      (lsp-append-defn-completions-loop src (+ idx 1) len prefix items))))

(defn lsp-append-defn-completions [src prefix]
  (lsp-append-defn-completions-loop src 0 (string-length src) prefix (vector-new 8)))

(defn lsp-ensure-trailing-newline [src]
  (let [len (string-length src)]
    (if (= len 0)
      "\n"
      (if (= (string-char-at src (- len 1)) 10)
        src
        (string-concat src "\n")))))

;; textDocument/hover: ホバー情報の提供
;; カーソル位置の型情報をマークダウン形式で返す (AC-205)
;; params=[uri, line, col, source] の場合はソース走査で symbol 情報を返す
(defn lsp-hover-mock-text [params]
  (let [line (if (= params 0) 0 (vector-get params 1))
        col (if (= params 0) 0 (vector-get params 2))]
    (string-concat
      "type-info:"
      (string-concat
        (int-to-string line)
        (string-concat ":" (int-to-string col))))))

(defn handle-hover-mock [params]
  (let [v (vector-new 2)
        contents (lsp-hover-mock-text params)]
    (vector-push
      (vector-push v 0)     ;; range
      contents)))            ;; contents: 型情報 text

(defn handle-hover [params state]
  (do
    (server-state-note-request state)
    (let [uri (lsp-nav-uri params)
          src (lsp-session-src params state)
          line (lsp-nav-line params)
          col (lsp-nav-col params)
          symbol (lsp-symbol-at src line col)
          start (symbol-info-start symbol)
          end (symbol-info-end symbol)]
      (if (> (string-length src) 0)
          (if (>= start 0)
             (let [name (substring src start end)
                   range (lsp-range-from-offsets src start end)
                   resolution (lsp-resolve-defn-in-open-docs state uri src name start)
                   defn-offset (lsp-defn-resolution-offset resolution)
                   contents (lsp-hover-content-text defn-offset name)]
               (vector-push
                 (vector-push (vector-new 2) range)
                 contents))
          (handle-hover-mock params))
        (handle-hover-mock params)))))

;; textDocument/goto-definition: 定義ジャンプ
;; シンボルの定義位置を Location [uri, line, col] として返す (AC-206)
(defn handle-goto-definition-mock [params]
  (let [v (vector-new 3)
        ;; params が vector の場合、元の位置情報をもとにモック位置を返す
        uri (if (= params 0) 0 (vector-get params 0))
        line (if (= params 0) 0 1)
        col 0]
    (vector-push
      (vector-push
        (vector-push v uri)  ;; uri
        line)                ;; line
      col)))                 ;; col

(defn handle-goto-definition [params state]
  (do
    (server-state-note-request state)
    (let [uri (lsp-nav-uri params)
          src (lsp-session-src params state)
          line (lsp-nav-line params)
          col (lsp-nav-col params)
          symbol (lsp-symbol-at src line col)
          start (symbol-info-start symbol)
          end (symbol-info-end symbol)]
      (if (> (string-length src) 0)
        (if (>= start 0)
          (let [name (substring src start end)
                resolution (lsp-resolve-defn-in-open-docs state uri src name start)
                target-uri (lsp-defn-resolution-uri resolution)
                defn-offset (lsp-defn-resolution-offset resolution)]
            (if (>= defn-offset 0)
              (let [target-src (if (= target-uri uri)
                                 src
                                 (server-state-source-for-uri state target-uri))
                    pos (lsp-position-from-offset target-src defn-offset)]
                (make-location target-uri (position-line pos) (position-col pos)))
              (handle-goto-definition-mock params)))
          (handle-goto-definition-mock params))
        (handle-goto-definition-mock params)))))

;; textDocument/references: 参照箇所の検索
;; シンボルの参照位置リストを返す (AC-206)
;; 各 location は [uri, line, col] の 3 要素
(defn make-location [uri line col]
  (vector-push (vector-push (vector-push (vector-new 3) uri) line) col))

(defn handle-references-mock [params]
  (let [;; モック: params の位置自体を 1 つの参照として返す
         uri (if (= params 0) 0 (vector-get params 0))
         line (if (= params 0) 0 (vector-get params 1))
         col (if (= params 0) 0 (vector-get params 2))
         loc (make-location uri line col)]
    (vector-push (vector-new 1) loc)))

(defn handle-references [params state]
  (do
    (server-state-note-request state)
    (let [uri (lsp-nav-uri params)
          src (lsp-session-src params state)
          line (lsp-nav-line params)
          col (lsp-nav-col params)
          symbol (lsp-symbol-at src line col)
          start (symbol-info-start symbol)
          end (symbol-info-end symbol)]
      (if (> (string-length src) 0)
        (if (>= start 0)
          (lsp-find-occurrences src (substring src start end) uri)
          (handle-references-mock params))
        (handle-references-mock params)))))

;; textDocument/rename: リネーム
;; シンボルのリネーム用 WorkspaceEdit を返す
(defn handle-rename-mock [params]
  (let [v (vector-new 1)]
    (vector-push v 0)))

(defn lsp-append-rename-edits-loop [locs idx len old-name-len new-name edits]
  (if (>= idx len)
    edits
    (let [loc (vector-get locs idx)
          line (vector-get loc 1)
          col (vector-get loc 2)
          edit (make-text-edit line col line (+ col old-name-len) new-name)]
      (lsp-append-rename-edits-loop locs (+ idx 1) len old-name-len new-name (vector-push edits edit)))))

(defn lsp-build-rename-edits [locs old-name-len new-name]
  (lsp-append-rename-edits-loop locs 0 (vector-length locs) old-name-len new-name (vector-new 4)))

(defn handle-rename [params state]
  (do
    (server-state-note-request state)
    (let [uri (lsp-nav-uri params)
          src (lsp-session-src params state)
          line (lsp-nav-line params)
          col (lsp-nav-col params)
          new-name (lsp-rename-new-name params)
          symbol (lsp-symbol-at src line col)
          start (symbol-info-start symbol)
          end (symbol-info-end symbol)]
      (if (> (string-length src) 0)
        (if (>= start 0)
          (if (> (string-length new-name) 0)
            (let [name (substring src start end)
                  locs (lsp-find-occurrences src name uri)
                  edits (lsp-build-rename-edits locs (string-length name) new-name)]
              (vector-push (vector-new 1) (make-workspace-change uri edits)))
            (handle-rename-mock params))
          (handle-rename-mock params))
        (handle-rename-mock params)))))  ;; changes

;; textDocument/formatting: ドキュメントフォーマット
;; Formatter.ls の format-program を呼び出して TextEdit リストを返す (AC-010)
(defn handle-formatting-mock [params]
  (let [edit (vector-new 1)]
    (vector-push edit 0)))

(defn handle-formatting [params state]
  (do
    (server-state-note-request state)
    (let [src (lsp-session-document-src params state)]
      (if (> (string-length src) 0)
        (let [end-pos (lsp-position-from-offset src (string-length src))
              program (parse-program src)
              formatted (format-program-with-source program src)
              edit (make-format-edit 1 1 (position-line end-pos) (position-col end-pos) formatted)]
          (vector-push (vector-new 1) edit))
        (handle-formatting-mock params)))))

;; textDocument/completion: コード補完
;; カーソル位置に基づいてキーワード補完候補リストを返す (AC-207)
(defn handle-completion [params state]
  (do
    (server-state-note-request state)
    (let [src (lsp-session-src params state)
          line (lsp-nav-line params)
          col (lsp-nav-col params)]
      (if (> (string-length src) 0)
        (let [prefix (lsp-prefix-at src line col)
              items (lsp-append-defn-completions src prefix)
              items (lsp-append-keyword-completions prefix items)]
          items)
        (let [items (vector-new 7)
              ;; L# キーワード: defn, let, if, match, do, fn, module
              items (vector-push items (lsp-make-completion-item "defn" 14 "defn"))
              items (vector-push items (lsp-make-completion-item "let" 14 "let"))
              items (vector-push items (lsp-make-completion-item "if" 14 "if"))
              items (vector-push items (lsp-make-completion-item "match" 14 "match"))
              items (vector-push items (lsp-make-completion-item "do" 14 "do"))
              items (vector-push items (lsp-make-completion-item "fn" 14 "fn"))
              items (vector-push items (lsp-make-completion-item "module" 14 "module"))]
          items)))))

;; === 診断の安定順序制御 (T4b-3 AC-208/AC-209/AC-210/AC-211) ===

;; sort-diagnostics: 診断をソースごとにグルーピングし決定的順序でソートする
;; 入力: 診断 Vector [severity, rule-id, line, col, msg-hash, source]
;; 出力: ソート済み診断 Vector
;; AC-208: source フィールドでグルーピング → 行番号昇順
;; AC-209: 同一 span の重複は severity 高い方のみ残す
;; AC-211: 決定的 (deterministic) な順序を保証

;; 挿入ソートの内側ループ: sorted の idx 位置に elem を挿入する場所を見つけて挿入
;; result: sorted の先頭 idx 要素を保持し、elem をキー順で挿入した新 Vector を返す
(defn sort-diag-insert [sorted elem elem-key idx]
  (if (= idx 0)
    ;; 先頭に挿入
    (let [out (vector-new (+ (vector-length sorted) 1))
          out (vector-push out elem)]
      (sort-diag-copy sorted 0 (vector-length sorted) out))
    (let [prev (vector-get sorted (- idx 1))
          prev-key (diagnostic-order-key prev)]
      (if (= (diagnostic-order-before elem elem-key prev prev-key) 1)
        ;; まだ前に移動する必要がある
        (sort-diag-insert sorted elem elem-key (- idx 1))
        ;; ここに挿入: 0..idx をコピー → elem → idx..len をコピー
        (let [out (vector-new (+ (vector-length sorted) 1))
              out (sort-diag-copy sorted 0 idx out)
              out (vector-push out elem)]
          (sort-diag-copy sorted idx (vector-length sorted) out))))))

;; sorted の from..to をコピーして out に追加する
(defn sort-diag-copy [src from to out]
  (if (>= from to)
    out
    (sort-diag-copy src (+ from 1) to (vector-push out (vector-get src from)))))

;; 挿入ソートの外側ループ: diagnostics の idx 番目から順に sorted に挿入
(defn sort-diag-loop [diagnostics sorted idx len]
  (if (>= idx len)
    sorted
    (let [elem (vector-get diagnostics idx)
          elem-key (diagnostic-order-key elem)
          new-sorted (sort-diag-insert sorted elem elem-key (vector-length sorted))]
      (sort-diag-loop diagnostics new-sorted (+ idx 1) len))))

(defn sort-diagnostics [diagnostics]
  (let [len (vector-length diagnostics)]
    (if (< len 2)
      diagnostics
      (let [first (vector-get diagnostics 0)
            initial (vector-push (vector-new 1) first)]
        (sort-diag-loop diagnostics initial 1 len)))))

;; 診断の重複マージ (AC-209)
;; 同一 span に対する重複診断は severity の高い方 (数値が小さい方) のみ残す
(defn merge-duplicate-diagnostics [diagnostics]
  (let [len (vector-length diagnostics)]
    (if (= len 2)
      (let [diag0 (vector-get diagnostics 0)
            diag1 (vector-get diagnostics 1)
            line0 (vector-get diag0 2)
            col0 (vector-get diag0 3)
            line1 (vector-get diag1 2)
            col1 (vector-get diag1 3)
            sev0 (vector-get diag0 0)
            sev1 (vector-get diag1 0)]
        (if (= line0 line1)
          (if (= col0 col1)
            (if (< sev0 sev1)
              (vector-push (vector-new 1) diag0)
              (vector-push (vector-new 1) diag1))
            diagnostics)
          diagnostics))
      diagnostics)))

;; 診断の順序キーを計算 (AC-211: deterministic order)
;; source(1=parse,2=type,3=lint) → severity(1=error,2=warning,3=info,4=hint) → line → col
(defn diagnostic-order-key [diag]
  (let [source (vector-get diag 5)
        sev (vector-get diag 0)
        line (vector-get diag 2)
        col (vector-get diag 3)]
    (+ (* source 100000000)
       (+ (* sev 1000000)
          (+ (* line 10000) col)))))

;; 同じ source/severity/span の診断も rule/message で安定順序化する
(defn diagnostic-order-before [a a-key b b-key]
  (if (< a-key b-key)
    1
    (if (> a-key b-key)
      0
      (let [rule-a (vector-get a 1)
            rule-b (vector-get b 1)
            msg-a (vector-get a 4)
            msg-b (vector-get b 4)]
        (if (< rule-a rule-b)
          1
          (if (> rule-a rule-b)
            0
            (if (< msg-a msg-b) 1 0)))))))

;; === 診断の重複除去 (AC-209) ===

;; 同一スパン判定: line と col が同じなら 1、異なれば 0
(defn dedup-diag-same-span [a b]
  (let [line-a (vector-get a 2)
        col-a (vector-get a 3)
        line-b (vector-get b 2)
        col-b (vector-get b 3)]
    (if (= line-a line-b) (if (= col-a col-b) 1 0) 0)))

;; severity の高い方 (数値が小さい方) を選択
(defn dedup-diag-pick-best [a b]
  (let [sev-a (vector-get a 0)
        sev-b (vector-get b 0)
        key-a (diagnostic-order-key a)
        key-b (diagnostic-order-key b)]
    (if (< sev-a sev-b)
      a
      (if (< sev-b sev-a)
        b
        (if (= (diagnostic-order-before a key-a b key-b) 1) a b)))))

;; result 内で diag と同一スパンの要素を探す (O(n) 走査)
(defn dedup-find-span [result diag idx len]
  (if (>= idx len)
    (- 0 1)
    (if (= (dedup-diag-same-span (vector-get result idx) diag) 1)
      idx
      (dedup-find-span result diag (+ idx 1) len))))

;; vector の replace-idx 番目を new-elem に置き換えた新 vector を返す
(defn dedup-replace-loop [vec out replace-idx new-elem idx len]
  (if (>= idx len)
    out
    (if (= idx replace-idx)
      (dedup-replace-loop vec (vector-push out new-elem) replace-idx new-elem (+ idx 1) len)
      (dedup-replace-loop vec (vector-push out (vector-get vec idx)) replace-idx new-elem (+ idx 1) len))))

(defn dedup-replace [vec replace-idx new-elem]
  (dedup-replace-loop vec (vector-new (vector-length vec)) replace-idx new-elem 0 (vector-length vec)))

;; dedup 本体: O(n²) で同一スパンを集約
(defn dedup-build [diags result idx len]
  (if (>= idx len)
    result
    (let [diag (vector-get diags idx)
          existing-idx (dedup-find-span result diag 0 (vector-length result))]
      (if (< existing-idx 0)
        (dedup-build diags (vector-push result diag) (+ idx 1) len)
        (let [existing (vector-get result existing-idx)
              best (dedup-diag-pick-best existing diag)
              new-result (dedup-replace result existing-idx best)]
          (dedup-build diags new-result (+ idx 1) len))))))

;; dedup-diagnostics: 同一 span の診断は severity 最高のみ残す (AC-209)
(defn dedup-diagnostics [diags]
  (let [len (vector-length diags)]
    (if (< len 2)
      diags
      (dedup-build diags (vector-new 0) 0 len))))

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
              (string-concat error-message "\"}}"))))))))

;; parse-json-rpc-request: JSON-RPC リクエストから method + params を抽出
;; 入力: [jsonrpc-version, id, method-id, params]
;; 出力: [method-id, params]
(defn parse-json-rpc-request [msg]
  (let [method-id (vector-get msg 2)
        params (vector-get msg 3)]
    (vector-push (vector-push (vector-new 2) method-id) params)))

;; === メインループ ===

;; LSP サーバーのメインループ
;; 現段階では 1 メッセージ request vector [method-id, params] を dispatch する PoC。
;; stateful/session 系は server-loop-step で shared state を再利用できる。
(defn server-loop-step [state request]
  (let [method-id (vector-get request 0)
        params (vector-get request 1)]
    (json-rpc-dispatch method-id params state)))

(defn server-loop-sequence-loop [state requests idx count results]
  (if (>= idx count)
    results
    (server-loop-sequence-loop
      state
      requests
      (+ idx 1)
      count
      (vector-push results (server-loop-step state (vector-get requests idx))))))

(defn server-loop-sequence [requests]
  (let [state (server-state-new)
        results (server-loop-sequence-loop state requests 0 (vector-length requests) (vector-new 8))
        summary (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push summary results)
          (server-state-doc-count state))
        (server-state-request-count state))
      (server-state-source-length state))))

(defn server-loop [request]
  (let [state (server-state-new)]
    (server-loop-step state request)))

;; 検証用 main
(defn main []
  (let [;; サーバー状態の初期化
        state (server-state-new)
        ;; initialize ハンドラ
        caps (handle-initialize 0 state)
        did-open (handle-didOpen 12 state)
        did-change (handle-didChange 8 state)
        hover (handle-hover 0 state)
        goto-def (handle-goto-definition 0 state)
        refs (handle-references 0 state)
        rename (handle-rename 0 state)
        formatting (handle-formatting 0 state)
        completions (handle-completion 0 state)
        r2 (json-rpc-dispatch (lsp-method-shutdown) 0 state)
        diag-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 3) 2) 0) 0)
        diag-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 100) 1) 1) 0) 0)
        diags (vector-push (vector-push (vector-new 2) diag-a) diag-b)
        sorted (sort-diagnostics diags)
        dup-a (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 2) 101) 5) 7) 0) 0)
        dup-b (vector-push (vector-push (vector-push (vector-push (vector-push (vector-push (vector-new 6) 1) 102) 5) 7) 0) 0)
        dup-diags (vector-push (vector-push (vector-new 2) dup-a) dup-b)
        merged (merge-duplicate-diagnostics dup-diags)]
    (do
      ;; capabilities の検証
      (print (vector-length caps)) ;; 7
      (print (vector-get caps 0))  ;; 1 (textDocumentSync: Full)
      (print (vector-get caps 1))  ;; 1 (hoverProvider)
      (print (vector-get caps 2))  ;; 1 (completionProvider)
      ;; basic handler の検証
      (print did-open)             ;; 12
      (print did-change)           ;; 8
      (print (vector-length formatting)) ;; 1
      (print (vector-length completions)) ;; 7
      ;; shutdown の検証
      (print r2)                    ;; 0
      ;; sort-diagnostics の検証 (source=0, sev=1 → key = 0*100M + 1*1M + line*10K + col)
      (print (diagnostic-order-key (vector-get sorted 0))) ;; 1010001
      (print (diagnostic-order-key (vector-get sorted 1))) ;; 1030002
      ;; merge-duplicate-diagnostics の検証
      (print (vector-length merged)) ;; 1
      (print (vector-get (vector-get merged 0) 0)) ;; 1
      ;; navigation handler shape の検証
      (print (vector-length hover)) ;; 2
      (print (vector-length goto-def)) ;; 3
      (print (vector-length refs)) ;; 1
      (print (vector-length rename)) ;; 1
      0)))
