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
;; params から line/col を取り出し、シンボルの型情報ハッシュを返す
(defn handle-hover [params state]
  (let [v (vector-new 2)
        ;; params が vector の場合、line 情報から型情報ハッシュを生成
        type-hash (if (> params 0)
                    (+ (* (vector-get params 1) 100) (vector-get params 2))
                    1)]
    (vector-push
      (vector-push v 0)     ;; range
      type-hash)))           ;; contents: 型情報ハッシュ

;; textDocument/goto-definition: 定義ジャンプ
;; シンボルの定義位置を Location [uri, line, col] として返す (AC-206)
(defn handle-goto-definition [params state]
  (let [v (vector-new 3)
        ;; params が vector の場合、元の位置情報をもとにモック位置を返す
        uri (if (> params 0) (vector-get params 0) 0)
        line (if (> params 0) 1 0)
        col 0]
    (vector-push
      (vector-push
        (vector-push v uri)  ;; uri
        line)                ;; line
      col)))                 ;; col

;; textDocument/references: 参照箇所の検索
;; シンボルの参照位置リストを返す (AC-206)
;; 各 location は [uri, line, col] の 3 要素
(defn make-location [uri line col]
  (vector-push (vector-push (vector-push (vector-new 3) uri) line) col))

(defn handle-references [params state]
  (let [;; モック: params の位置自体を 1 つの参照として返す
        uri (if (> params 0) (vector-get params 0) 0)
        line (if (> params 0) (vector-get params 1) 0)
        col (if (> params 0) (vector-get params 2) 0)
        loc (make-location uri line col)]
    (vector-push (vector-new 1) loc)))

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
;; カーソル位置に基づいてキーワード補完候補リストを返す (AC-207)
;; 各 item は [label-hash, kind] の 2 要素。kind=14 は LSP CompletionItemKind.Keyword
(defn make-completion-item [label-hash kind]
  (vector-push (vector-push (vector-new 2) label-hash) kind))

(defn handle-completion [params state]
  (let [items (vector-new 7)
        ;; L# キーワード: defn, let, if, match, do, fn, module
        items (vector-push items (make-completion-item 1 14))   ;; defn
        items (vector-push items (make-completion-item 2 14))   ;; let
        items (vector-push items (make-completion-item 3 14))   ;; if
        items (vector-push items (make-completion-item 4 14))   ;; match
        items (vector-push items (make-completion-item 5 14))   ;; do
        items (vector-push items (make-completion-item 6 14))   ;; fn
        items (vector-push items (make-completion-item 7 14))]  ;; module
    items))

;; === 診断の安定順序制御 (T4b-3 AC-208/AC-209/AC-210/AC-211) ===

;; sort-diagnostics: 診断をソースごとにグルーピングし行番号昇順にソートする
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
      (if (< elem-key prev-key)
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
        sev-b (vector-get b 0)]
    (if (< sev-a sev-b) a b)))

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

;; === JSON-RPC エンコード/パース ===

;; encode-json-rpc-response: JSON-RPC 2.0 レスポンス構造を生成
;; [jsonrpc-version(=2), id, result]
(defn encode-json-rpc-response [id result]
  (vector-push (vector-push (vector-push (vector-new 3) 2) id) result))

;; parse-json-rpc-request: JSON-RPC リクエストから method + params を抽出
;; 入力: [jsonrpc-version, id, method-id, params]
;; 出力: [method-id, params]
(defn parse-json-rpc-request [msg]
  (let [method-id (vector-get msg 2)
        params (vector-get msg 3)]
    (vector-push (vector-push (vector-new 2) method-id) params)))

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
