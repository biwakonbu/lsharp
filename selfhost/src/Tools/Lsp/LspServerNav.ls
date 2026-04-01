(module Tools.Lsp.LspServerNav)
(import Syntax.Parser)
(import Tools.Text.FormatterDecl)
(import Tools.Lsp.JsonRpc)
(import Tools.Lsp.LspServerCore)

;; LspServerNav.ls - LSP ナビゲーション・補完・シンボル解析
;;
;; JSON レンダリング (ナビゲーション部)、位置/オフセット変換、
;; シンボル走査、定義解決、出現箇所検索、補完候補生成、
;; ナビゲーションハンドラ (hover/goto-definition/references/rename/
;; formatting/completion)、診断ソート/重複除去を含む。
;;
;; コア機能 (状態管理・ディスパッチ等) は LspServerCore.ls に分離。

;; === JSON レンダリング (ナビゲーション部) ===

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
    label-json (json-escape-string label)
    kind-text (int-to-string (vector-get item 1))
    insert-text (vector-get item 2)
    insert-json (json-escape-string insert-text)
    payload-0 "[\""
    payload-1 (string-concat payload-0 label-json)
    payload-2 (string-concat payload-1 "\",")
    payload-3 (string-concat payload-2 kind-text)
    payload-4 (string-concat payload-3 ",\"")
    payload-5 (string-concat payload-4 insert-json)]
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
    payload-9 (string-concat payload-8 (json-escape-string (vector-get edit 4)))]
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

;; === 位置/オフセット変換 ===

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

;; === シンボル走査 ===

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

;; === 定義解決 ===

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

;; === 補完・プレフィックスマッチ ===

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

(defn lsp-completion-item-label [item]
  (vector-get item 0))

(defn lsp-completion-has-label-loop [items label idx len]
  (if (>= idx len)
    0
    (if (string-eq (lsp-completion-item-label (vector-get items idx)) label)
      1
      (lsp-completion-has-label-loop items label (+ idx 1) len))))

(defn lsp-completion-push-unique [items item]
  (if (= (lsp-completion-has-label-loop items (lsp-completion-item-label item) 0 (vector-length items)) 1)
    items
    (vector-push items item)))

(defn lsp-merge-completion-items-loop [items extra idx len]
  (if (>= idx len)
    items
    (lsp-merge-completion-items-loop
      (lsp-completion-push-unique items (vector-get extra idx))
      extra
      (+ idx 1)
      len)))

(defn lsp-merge-completion-items [items extra]
  (lsp-merge-completion-items-loop items extra 0 (vector-length extra)))

(defn lsp-append-open-doc-completions-loop [state current-uri prefix idx count items]
  (if (>= idx count)
    items
    (let [target-uri (vector-get (server-state-uri-list state) idx)]
      (if (= target-uri current-uri)
        (lsp-append-open-doc-completions-loop state current-uri prefix (+ idx 1) count items)
        (let [target-src (server-state-source-for-uri state target-uri)
          target-items (lsp-append-defn-completions target-src prefix)]
          (lsp-append-open-doc-completions-loop
            state
            current-uri
            prefix
            (+ idx 1)
            count
            (lsp-merge-completion-items items target-items)))))))

(defn lsp-append-open-doc-completions [state current-uri prefix items]
  (lsp-append-open-doc-completions-loop
    state
    current-uri
    prefix
    0
    (vector-length (server-state-uri-list state))
    items))

(defn lsp-ensure-trailing-newline [src]
  (let [len (string-length src)]
    (if (= len 0)
      "\n"
      (if (= (string-char-at src (- len 1)) 10)
        src
        (string-concat src "\n")))))

;; === ナビゲーションハンドラ ===

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
      (vector-push v 0) ;; range
      contents))) ;; contents: 型情報 text

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
        (vector-push v uri) ;; uri
        line) ;; line
      col))) ;; col

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
        (handle-rename-mock params))))) ;; changes

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
          items (lsp-append-open-doc-completions state (lsp-nav-uri params) prefix items)
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
