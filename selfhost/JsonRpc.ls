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

;; JSON-RPC メッセージ構築
;; [type, id, method-hash, params-count]
(defn make-rpc-request [id method-hash param-count]
  (let [v (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push v 0)
          id)
        method-hash)
      param-count)))

(defn make-rpc-response [id result]
  (let [v (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push v 1)
        id)
      result)))

(defn make-rpc-notification [method-hash]
  (let [v (vector-new 2)]
    (vector-push
      (vector-push v 2)
      method-hash)))

(defn make-rpc-error [id error-code]
  (let [v (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push v 3)
        id)
      error-code)))

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

;; Content-Length ヘッダーのパース (簡易実装)
;; 実際の LSP では "Content-Length: N\r\n\r\n" の N を数値化する
(defn parse-content-length [header-value]
  header-value)

;; === P9-6b: LSP ハンドラ実装 ===

;; 追加 LSP メソッド定数
(defn method-formatting [] 23)
(defn method-publish-diagnostics [] 30)

;; サーバー capabilities: [sync, hover, completion, goto-def, formatting]
(defn make-server-capabilities []
  (let [v (vector-new 5)]
    (vector-push (vector-push (vector-push (vector-push (vector-push v 1) 1) 1) 1) 1)))

;; handle-initialize: 初期化リクエスト → capabilities レスポンス
(defn handle-initialize [request-id]
  (make-rpc-response request-id 1))

;; handle-did-open: ドキュメントオープン通知 → ソース長を返す
(defn handle-did-open [source-length]
  source-length)

;; handle-did-change: ドキュメント変更通知 → ソース長を返す
(defn handle-did-change [source-length]
  source-length)

;; handle-hover: 型ホバー → 型タグをレスポンスで返す
(defn handle-hover [request-id type-tag]
  (make-rpc-response request-id type-tag))

;; handle-goto-def: 定義ジャンプ → [line, col] をレスポンスで返す
(defn handle-goto-def [request-id line col]
  (let [pos (vector-new 2)]
    (make-rpc-response request-id (vector-push (vector-push pos line) col))))

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
      (print (rpc-type req))      ;; 0 (request)
      (print (rpc-type resp))     ;; 1 (response)
      (print (rpc-type notif))    ;; 2 (notification)
      (print (rpc-type err))      ;; 3 (error)

      ;; ID の検証
      (print (rpc-id req))        ;; 1
      (print (rpc-id resp))       ;; 1
      (print (rpc-id err))        ;; 1

      ;; メソッドハッシュの検証
      (print (method-initialize)) ;; 1
      (print (method-shutdown))   ;; 2

      ;; === P9-6b: LSP ハンドラ検証 ===

      ;; server capabilities
      (let [caps (make-server-capabilities)]
        (do
          (print (vector-length caps))   ;; 5
          (print (vector-get caps 0))))  ;; 1 (text-document-sync)

      ;; handle-initialize
      (let [init-resp (handle-initialize 1)]
        (do
          (print (rpc-type init-resp))   ;; 1 (response)
          (print (rpc-id init-resp))))   ;; 1

      ;; handle-did-open
      (print (handle-did-open 100))      ;; 100

      ;; handle-hover
      (let [hover-resp (handle-hover 2 1)]
        (do
          (print (rpc-type hover-resp))  ;; 1 (response)
          (print (rpc-id hover-resp))))  ;; 2

      ;; handle-goto-def
      (let [def-resp (handle-goto-def 3 10 5)]
        (do
          (print (rpc-type def-resp))    ;; 1 (response)
          (let [def-pos (vector-get def-resp 2)]
            (do
              (print (vector-get def-pos 0))  ;; 10 (line)
              (print (vector-get def-pos 1))))))  ;; 5 (col)

      ;; handle-completion
      (print (make-keyword-completions))  ;; 7

      ;; 追加メソッド定数
      (print (method-formatting))         ;; 23
      (print (method-publish-diagnostics)) ;; 30

      0)))
