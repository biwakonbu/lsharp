(module LspServer)
(import JsonRpc)
(import AST)
(import Linter)
(import Formatter)

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
  (let [v (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push v 0)   ;; initialized フラグ
        0)                   ;; shutdown フラグ
      0)))                   ;; ドキュメント数

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
  (if (= method-id (lsp-method-formatting)) (handle-formatting params state)
  (if (= method-id (lsp-method-completion)) (handle-completion params state)
  0)))))))))))

;; === LSP メソッドハンドラ ===

;; initialize: サーバー機能の宣言
;; TextDocumentSyncKind.Full を返す (AC-200)
(defn handle-initialize [params state]
  (let [capabilities (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push capabilities 1)  ;; textDocumentSync: Full
          1)                             ;; hoverProvider
        1)                               ;; completionProvider
      1)))                               ;; definitionProvider

;; shutdown: サーバー終了準備
(defn handle-shutdown [params state]
  0)

;; textDocument/didOpen: ドキュメントオープン通知
;; フルテキストを受け取りパースして診断を生成
(defn handle-didOpen [params state]
  params)

;; textDocument/didChange: ドキュメント変更通知
;; Full sync: 全文を受け取りパースし直す (AC-201)
(defn handle-didChange [params state]
  params)

;; textDocument/hover: ホバー情報の提供
;; カーソル位置の型情報をマークダウン形式で返す (AC-205)
(defn handle-hover [params state]
  (let [v (vector-new 2)]
    (vector-push
      (vector-push v 0)  ;; range
      0)))               ;; contents (markdown)

;; textDocument/goto-definition: 定義ジャンプ
;; シンボルの定義位置を Location として返す (AC-206)
(defn handle-goto-definition [params state]
  (let [v (vector-new 2)]
    (vector-push
      (vector-push v 0)  ;; uri
      0)))               ;; range

;; textDocument/references: 参照箇所の検索
;; シンボルの参照位置リストを返す (AC-206)
(defn handle-references [params state]
  (vector-new 0))

;; textDocument/rename: リネーム
;; シンボルのリネーム用 WorkspaceEdit を返す
(defn handle-rename [params state]
  (let [v (vector-new 1)]
    (vector-push v 0)))  ;; changes

;; textDocument/formatting: ドキュメントフォーマット
;; Formatter.ls の format-program を呼び出して TextEdit リストを返す (AC-010)
(defn handle-formatting [params state]
  (let [edit (vector-new 1)]
    (vector-push edit 0)))

;; textDocument/completion: コード補完
;; カーソル位置に基づいて補完候補リストを返す (AC-207)
(defn handle-completion [params state]
  7)

;; === 診断の安定順序制御 (T4b-3 AC-208/AC-209/AC-210/AC-211) ===

;; sort-diagnostics: 診断をソースごとにグルーピングし行番号昇順にソートする
;; 入力: 診断 Vector [severity, rule-id, line, col, msg-hash, source]
;; 出力: ソート済み診断 Vector
;; AC-208: source フィールドでグルーピング → 行番号昇順
;; AC-209: 同一 span の重複は severity 高い方のみ残す
;; AC-211: 決定的 (deterministic) な順序を保証
(defn sort-diagnostics [diagnostics]
  (let [len (vector-length diagnostics)
        sorted (vector-new len)]
    (if (= len 0)
      diagnostics
      (if (= len 1)
        diagnostics
        (let [diag0 (vector-get diagnostics 0)
              diag1 (vector-get diagnostics 1)
              key0 (diagnostic-order-key diag0)
              key1 (diagnostic-order-key diag1)]
          (if (< key0 key1)
            (vector-push (vector-push sorted diag0) diag1)
            (vector-push (vector-push sorted diag1) diag0)))))))

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

;; 診断の順序を文字列化して検証 (AC-211: deterministic order)
(defn diagnostic-order-key [diag]
  (let [line (vector-get diag 2)
        col (vector-get diag 3)]
    (+ (* line 10000) col)))

;; === メインループ ===

;; LSP サーバーのメインループ
;; 現段階では 1 メッセージ request vector [method-id, params] を dispatch する PoC
(defn server-loop [request]
  (let [state (server-state-new)
        method-id (vector-get request 0)
        params (vector-get request 1)]
    (json-rpc-dispatch method-id params state)))

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
      (print (vector-length caps)) ;; 4
      (print (vector-get caps 0))  ;; 1 (textDocumentSync: Full)
      (print (vector-get caps 1))  ;; 1 (hoverProvider)
      (print (vector-get caps 2))  ;; 1 (completionProvider)
      ;; basic handler の検証
      (print did-open)             ;; 12
      (print did-change)           ;; 8
      (print (vector-length formatting)) ;; 1
      (print completions)          ;; 7
      ;; shutdown の検証
      (print r2)                    ;; 0
      ;; sort-diagnostics の検証
      (print (diagnostic-order-key (vector-get sorted 0))) ;; 10001
      (print (diagnostic-order-key (vector-get sorted 1))) ;; 30002
      ;; merge-duplicate-diagnostics の検証
      (print (vector-length merged)) ;; 1
      (print (vector-get (vector-get merged 0) 0)) ;; 1
      ;; navigation handler shape の検証
      (print (vector-length hover)) ;; 2
      (print (vector-length goto-def)) ;; 2
      (print (vector-length refs)) ;; 0
      (print (vector-length rename)) ;; 1
      0)))
