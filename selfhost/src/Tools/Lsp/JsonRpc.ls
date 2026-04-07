(module Tools.Lsp.JsonRpc)

;; JsonRpc.ls - JSON-RPC パーサー/シリアライザー
;;
;; P9-6b: LSP サーバーの基盤となる JSON-RPC プロトコル処理
;; LSP メッセージは JSON-RPC 2.0 形式:
;; {"jsonrpc":"2.0","id":1,"method":"initialize","params":{...}}
;;
;; 現在は整数タグ + Vector 方式で JSON-RPC メッセージを表現
;; (将来的には stdlib/Json.ls の完全 JSON パーサーと統合)

;; JSON-RPC メッセージ種別
(defn rpc-request [] 0)
(defn rpc-response [] 1)
(defn rpc-notification [] 2)
(defn rpc-error [] 3)
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

;; JSON-RPC メッセージ構築
;; [type, id, method-hash, params-count]
(defn make-rpc-request [id method-hash param-count]
  (let [v (vector-new 4)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local v 0)
          id)
        method-hash)
      param-count)))

(defn make-rpc-response [id result]
  (let [v (vector-new 3)]
    (push-object-vector-local
      (push-int-vector-local
        (push-int-vector-local v 1)
        id)
      result)))

(defn make-rpc-notification [method-hash]
  (let [v (vector-new 2)]
    (push-int-vector-local
      (push-int-vector-local v 2)
      method-hash)))

(defn make-rpc-error [id error-code]
  (let [v (vector-new 3)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local v 3)
        id)
      error-code)))

(defn render-int-vector-json-loop [values idx len out]
  (if (>= idx len)
    out
    (let [elem-text (int-to-string (vector-get values idx))
      next-out (if (= idx 0)
        (string-concat out elem-text)
        (string-concat out (string-concat "," elem-text)))]
      (render-int-vector-json-loop values (+ idx 1) len next-out))))

(defn render-int-vector-json [values]
  (string-concat "["
    (string-concat
      (render-int-vector-json-loop values 0 (vector-length values) "")
      "]")))

(defn json-hex-digit [digit]
  (if (= digit 0)
    "0"
    (if (= digit 1)
      "1"
      (if (= digit 2)
        "2"
        (if (= digit 3)
          "3"
          (if (= digit 4)
            "4"
            (if (= digit 5)
              "5"
              (if (= digit 6)
                "6"
                (if (= digit 7)
                  "7"
                  (if (= digit 8)
                    "8"
                    (if (= digit 9)
                      "9"
                      (if (= digit 10)
                        "a"
                        (if (= digit 11)
                          "b"
                          (if (= digit 12)
                            "c"
                            (if (= digit 13)
                              "d"
                              (if (= digit 14)
                                "e"
                                "f"))))))))))))))))

(defn json-control-escape [ch]
  (let [hi (/ ch 16)
    lo (- ch (* hi 16))]
    (string-concat
      "\\u00"
      (string-concat (json-hex-digit hi) (json-hex-digit lo)))))

(defn json-escape-char [src idx ch]
  (if (= ch 34)
    "\\\""
    (if (= ch 92)
      "\\\\"
      (if (= ch 10)
        "\\n"
        (if (= ch 13)
          "\\r"
          (if (= ch 9)
            "\\t"
            (if (= ch 8)
              "\\b"
              (if (= ch 12)
                "\\f"
                (if (< ch 32)
                  (json-control-escape ch)
                  (substring src idx (+ idx 1)))))))))))

(defn json-escape-string-loop [src idx len out]
  (if (>= idx len)
    out
    (let [ch (string-char-at src idx)
      piece (json-escape-char src idx ch)]
      (json-escape-string-loop src (+ idx 1) len (string-concat out piece)))))

(defn json-escape-string [src]
  (json-escape-string-loop src 0 (string-length src) ""))

(defn render-rpc-int-response [id result]
  (string-concat
    "{\"jsonrpc\":\"2.0\",\"id\":"
    (string-concat
      (int-to-string id)
      (string-concat
        ",\"result\":"
        (string-concat (int-to-string result) "}")))))

(defn render-rpc-int-vector-response [id result]
  (string-concat
    "{\"jsonrpc\":\"2.0\",\"id\":"
    (string-concat
      (int-to-string id)
      (string-concat
        ",\"result\":"
        (string-concat (render-int-vector-json result) "}")))))

(defn render-rpc-error-response [id error-code error-message]
  (let [message-json (json-escape-string error-message)]
    (string-concat
      "{\"jsonrpc\":\"2.0\",\"id\":"
      (string-concat
        (int-to-string id)
        (string-concat
          ",\"error\":{"
          (string-concat
            "\"code\":"
            (string-concat
              (int-to-string error-code)
              (string-concat
                ",\"message\":\""
                (string-concat message-json "\"}}")))))))))

;; JSON-RPC framing helpers
;; Content-Length は body 長から決定的に計算する
(defn render-content-length-header [payload]
  (let [len-text (int-to-string (string-length payload))]
    (string-concat
      "Content-Length: "
      (string-concat len-text "\r\n\r\n"))))

(defn render-json-rpc-frame [payload]
  (string-concat (render-content-length-header payload) payload))

(defn render-rpc-int-response-frame [id result]
  (render-json-rpc-frame (render-rpc-int-response id result)))

(defn render-rpc-int-vector-response-frame [id result]
  (render-json-rpc-frame (render-rpc-int-vector-response id result)))

(defn render-rpc-error-response-frame [id error-code error-message]
  (render-json-rpc-frame (render-rpc-error-response id error-code error-message)))

;; メッセージアクセサ
(defn rpc-type [msg]
  (vector-get msg 0))

(defn rpc-id [msg]
  (vector-get msg 1))

;; LSP メソッド名のハッシュ値 (簡易)
;; initialize = 1, shutdown = 2, textDocument/didOpen = 10,
;; textDocument/didChange = 11, textDocument/completion = 20
(defn method-initialize [] 1)
(defn method-shutdown [] 2)
(defn method-did-open [] 10)
(defn method-did-change [] 11)
(defn method-completion [] 20)
(defn method-hover [] 21)
(defn method-goto-def [] 22)
(defn method-formatting [] 23)
(defn method-references [] 24)
(defn method-rename [] 25)

;; Content-Length ヘッダーのパース (簡易実装)
;; 実際の LSP では "Content-Length: N\r\n\r\n" の N を数値化する
(defn parse-content-length-loop [header-value idx len acc started]
  (if (>= idx len)
    acc
    (let [c (string-char-at header-value idx)]
      (if (>= c 48)
        (if (<= c 57)
          (parse-content-length-loop
            header-value
            (+ idx 1)
            len
            (+ (* acc 10) (- c 48))
            1)
          (if (= started 1)
            acc
            (parse-content-length-loop header-value (+ idx 1) len acc started)))
        (if (= started 1)
          acc
          (parse-content-length-loop header-value (+ idx 1) len acc started))))))

(defn parse-content-length [header-value]
  (parse-content-length-loop header-value 0 (string-length header-value) 0 0))

;; === P9-6b: LSP ハンドラ実装 ===

;; 追加 LSP メソッド定数
(defn method-publish-diagnostics [] 30)

;; サーバー capabilities: [sync, hover, completion, goto-def, references, rename, formatting]
(defn make-server-capabilities []
  (let [v (vector-new 7)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local
              (push-int-vector-local
                (push-int-vector-local v 1) 1) 1) 1) 1) 1) 1)))

;; jsonrpc-handle-initialize: 初期化リクエスト → capabilities レスポンス
(defn jsonrpc-handle-initialize [request-id]
  (make-rpc-response request-id (make-server-capabilities)))

;; jsonrpc-handle-shutdown: shutdown リクエスト → sentinel result を返す
(defn jsonrpc-handle-shutdown [request-id]
  (make-rpc-response request-id 0))

(defn render-initialize-response [request-id]
  (render-rpc-int-vector-response request-id (make-server-capabilities)))

(defn render-shutdown-response [request-id]
  (render-rpc-int-response request-id 0))

(defn render-initialize-frame [request-id]
  (render-json-rpc-frame (render-initialize-response request-id)))

(defn render-shutdown-frame [request-id]
  (render-json-rpc-frame (render-shutdown-response request-id)))

;; handle-did-open: ドキュメントオープン通知 → ソース長を返す
(defn handle-did-open [source-length]
  source-length)

;; handle-did-change: ドキュメント変更通知 → ソース長を返す
(defn handle-did-change [source-length]
  source-length)

;; jsonrpc-handle-hover: 型ホバー → 型タグをレスポンスで返す
(defn jsonrpc-handle-hover [request-id type-tag]
  (make-rpc-response request-id type-tag))

;; handle-goto-def: 定義ジャンプ → [line, col] をレスポンスで返す
(defn handle-goto-def [request-id line col]
  (let [pos (vector-new 2)]
    (make-rpc-response request-id (push-int-vector-local (push-int-vector-local pos line) col))))

;; handle-completion: キーワード補完候補数
;; defn, let, if, match, do, fn, type = 7
(defn make-keyword-completions []
  7)

;; 検証用 main
(defn main []
  (let [req (make-rpc-request 1 (method-initialize) 0)
    resp (make-rpc-response 1 42)
    notif (make-rpc-notification (method-did-open))
    err (make-rpc-error 1 -32600)]
    (do
      ;; メッセージ種別の検証
      (print (rpc-type req)) ;; 0 (request)
      (print (rpc-type resp)) ;; 1 (response)
      (print (rpc-type notif)) ;; 2 (notification)
      (print (rpc-type err)) ;; 3 (error)

      ;; ID の検証
      (print (rpc-id req)) ;; 1
      (print (rpc-id resp)) ;; 1
      (print (rpc-id err)) ;; 1

      ;; メソッドハッシュの検証
      (print (method-initialize)) ;; 1
      (print (method-shutdown)) ;; 2

      ;; === P9-6b: LSP ハンドラ検証 ===

      ;; server capabilities
      (let [caps (make-server-capabilities)]
        (do
          (print (vector-length caps)) ;; 7
          (print (vector-get caps 0)))) ;; 1 (text-document-sync)

      ;; jsonrpc-handle-initialize
      (let [init-resp (jsonrpc-handle-initialize 1)]
        (do
          (print (rpc-type init-resp)) ;; 1 (response)
          (print (rpc-id init-resp))
          (print (vector-length (vector-get init-resp 2))))) ;; 1 / capabilities len

      ;; jsonrpc-handle-shutdown
      (let [shutdown-resp (jsonrpc-handle-shutdown 9)]
        (do
          (print (rpc-type shutdown-resp)) ;; 1 (response)
          (print (rpc-id shutdown-resp))
          (print (vector-get shutdown-resp 2)))) ;; 0 (result)

      ;; handle-did-open
      (print (handle-did-open 100)) ;; 100

      ;; jsonrpc-handle-hover
      (let [hover-resp (jsonrpc-handle-hover 2 1)]
        (do
          (print (rpc-type hover-resp)) ;; 1 (response)
          (print (rpc-id hover-resp)))) ;; 2

      ;; handle-goto-def
      (let [def-resp (handle-goto-def 3 10 5)]
        (do
          (print (rpc-type def-resp)) ;; 1 (response)
          (let [def-pos (vector-get def-resp 2)]
            (do
              (print (vector-get def-pos 0)) ;; 10 (line)
              (print (vector-get def-pos 1)))))) ;; 5 (col)

      ;; handle-completion
      (print (make-keyword-completions)) ;; 7

      ;; 追加メソッド定数
      (print (method-formatting)) ;; 23
      (print (method-publish-diagnostics)) ;; 30

      ;; deterministic JSON-RPC text
      (print-string (render-initialize-response 1))
      (print-string "\n")
      (print-string (render-shutdown-response 9))
      (print-string "\n")

      0)))
