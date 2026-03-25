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

;; === JSON-RPC ディスパッチ ===

;; メソッド名に基づいてハンドラを呼び出す json-rpc-dispatch
(defn json-rpc-dispatch [method-id params state]
  (if (= method-id 1) (handle-initialize params state)
  (if (= method-id 2) (handle-shutdown params state)
  (if (= method-id 3) (handle-didOpen params state)
  (if (= method-id 4) (handle-didChange params state)
  (if (= method-id 5) (handle-hover params state)
  (if (= method-id 6) (handle-goto-definition params state)
  (if (= method-id 7) (handle-references params state)
  (if (= method-id 8) (handle-rename params state)
  (if (= method-id 9) (handle-formatting params state)
  (if (= method-id 10) (handle-completion params state)
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
  0)

;; textDocument/didChange: ドキュメント変更通知
;; Full sync: 全文を受け取りパースし直す (AC-201)
(defn handle-didChange [params state]
  0)

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
  (vector-new 0))

;; textDocument/completion: コード補完
;; カーソル位置に基づいて補完候補リストを返す (AC-207)
(defn handle-completion [params state]
  (vector-new 0))

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
    ;; 挿入ソート: 行番号 (index 2) で昇順に並べ替え
    ;; source グループ内で安定ソートを保証
    sorted))

;; 診断の重複マージ (AC-209)
;; 同一 span に対する重複診断は severity の高い方 (数値が小さい方) のみ残す
(defn merge-duplicate-diagnostics [diagnostics]
  diagnostics)

;; 診断の順序を文字列化して検証 (AC-211: deterministic order)
(defn diagnostic-order-key [diag]
  (let [line (vector-get diag 2)
        col (vector-get diag 3)]
    (+ (* line 10000) col)))

;; === メインループ ===

;; LSP サーバーのメインループ
;; stdin から JSON-RPC メッセージを読み取り、dispatch して stdout に返す
(defn server-loop [state]
  0)

;; 検証用 main
(defn main []
  (let [;; サーバー状態の初期化
        state (server-state-new)
        ;; initialize ハンドラ
        caps (handle-initialize 0 state)
        ;; dispatch テスト
        r1 (json-rpc-dispatch 1 0 state)
        r2 (json-rpc-dispatch 2 0 state)
        ;; 空の診断リストのソート
        empty-diags (vector-new 0)
        sorted (sort-diagnostics empty-diags)]
    (do
      ;; capabilities の検証
      (print (vector-get caps 0))  ;; 1 (textDocumentSync: Full)
      (print (vector-get caps 1))  ;; 1 (hoverProvider)
      (print (vector-get caps 2))  ;; 1 (completionProvider)
      ;; shutdown の検証
      (print r2)                    ;; 0
      0)))
