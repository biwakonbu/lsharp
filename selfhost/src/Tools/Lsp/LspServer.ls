(module Tools.Lsp.LspServer)
(import Syntax.AST)
(import Syntax.Parser)
(import Tools.Text.Formatter)
(import Tools.Lsp.JsonRpc)
(import Tools.Lsp.LspServerCore)
(import Tools.Lsp.LspServerNav)

;; LspServer.ls - L# 製 LSP サーバー (ディスパッチャ/スタブ)
;;
;; P11-4 T4-2: L# 製 LSP の正式化
;; LSP 3.17 仕様に準拠した 10 メソッドを実装。
;; JSON-RPC 2.0 プロトコルによる通信。
;;
;; STR-02 分割:
;;   LspServerCore.ls - サーバー状態、ディスパッチ、ドキュメントハンドラ、
;;                      データ構造、JSON レンダリング (コア部)、メインループ
;;   LspServerNav.ls  - ナビゲーション、補完、シンボル解析、診断ソート/重複除去
;;   LspServer.ls     - エントリポイント (本ファイル)
;;
;; 対応メソッド:
;;   initialize, shutdown,
;;   textDocument/didOpen, textDocument/didChange,
;;   textDocument/hover, textDocument/definition,
;;   textDocument/references, textDocument/rename,
;;   textDocument/formatting, textDocument/completion
;;
;; バンドルモードでは LspServerCore.ls + LspServerNav.ls が
;; 全関数を提供し、本ファイルは dispatch / main を定義する。
;; diagnostic sort/order は LspServerNav.ls に実装。

;; === JSON-RPC ディスパッチ (STR-02 分割後もエントリに集約) ===
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

;; 各呼び出しで新規 state (旧単一ファイル実装と同じ)。共有は server-loop-step を直接使う。
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
      (print (vector-get caps 0)) ;; 1 (textDocumentSync: Full)
      (print (vector-get caps 1)) ;; 1 (hoverProvider)
      (print (vector-get caps 2)) ;; 1 (completionProvider)
      ;; basic handler の検証
      (print did-open) ;; 12
      (print did-change) ;; 8
      (print (vector-length formatting)) ;; 1
      (print (vector-length completions)) ;; 7
      ;; shutdown の検証
      (print r2) ;; 0
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
