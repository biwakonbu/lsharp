(module Tools.Doc.DocTools)
(import Syntax.AST)
(import Syntax.Parser)
(import Types.TypeScheme)
(import Types.TypeInferCore)
(import Types.TypeInfer)

;; DocTools.ls - L# 製ドキュメントツール
;;
;; ソース AST から決定的なドキュメント payload を組み立てる。
;; タイムスタンプ・ホスト名・絶対パスのような環境依存値は含めない。

;; === ドキュメント生成 ===

;; generate: ソース AST からドキュメント構造を生成
;; 出力: [title, body, functions, types]
(defn generate [ast opts]
  (let [functions (sort-doc-entries (extract-function-entries ast))
    types (sort-doc-entries (extract-type-entries ast))
    title (title-from-ast ast)
    body (make-doc-body-summary functions types)
    doc (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push doc title)
          body)
        functions)
      types)))

;; gen-doc: generate のエイリアス (互換用)
(defn gen-doc [ast]
  (generate ast 0))

;; doc-generate: generate のエイリアス (CLI 統合用)
(defn doc-generate [file-path opts]
  (filter-env-dependent
    (generate (parse-program (read-file file-path)) opts)))

;; doc-summary-size: 現在の deterministic doc structure の slot 数
(defn doc-summary-size [ast opts]
  4)

;; doc-file-summary-size: file-path 版の deterministic doc summary
(defn doc-file-summary-size [file-path opts]
  4)

;; === モジュールドキュメント抽出 ===

;; type 系の宣言タグかどうかを返す
(defn type-definition-tag? [tag]
  (if (= tag (ast-type-decl))
    1
    (if (= tag (ast-recorddef))
      1
      (if (= tag (ast-typealias))
        1
        (if (= tag (ast-typeconstrained))
          1
          0)))))

;; モジュール内の公開関数リストを抽出
(defn extract-public-functions [ast]
  (extract-public-functions-loop ast (vector-new 0) 0 (vector-length ast)))

(defn extract-public-functions-loop [ast result idx count]
  (if (>= idx count)
    result
    (let [decl (vector-get ast idx)
      next-result (extract-public-functions-decl decl result)]
      (extract-public-functions-loop ast next-result (+ idx 1) count))))

(defn extract-public-functions-decl [decl result]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (vector-push result decl)
      (if (= tag (ast-module-decl))
        (extract-public-functions-module-body decl result 0 (vector-get decl 2))
        result))))

(defn extract-public-functions-module-body [decl result idx count]
  (if (>= idx count)
    result
    (let [inner-decl (vector-get decl (+ idx 3))
      next-result (extract-public-functions-decl inner-decl result)]
      (extract-public-functions-module-body decl next-result (+ idx 1) count))))

;; モジュール内の型定義リストを抽出
(defn extract-type-definitions [ast]
  (extract-type-definitions-loop ast (vector-new 0) 0 (vector-length ast)))

(defn extract-type-definitions-loop [ast result idx count]
  (if (>= idx count)
    result
    (let [decl (vector-get ast idx)
      next-result (extract-type-definitions-decl decl result)]
      (extract-type-definitions-loop ast next-result (+ idx 1) count))))

(defn extract-type-definitions-decl [decl result]
  (let [tag (vector-get decl 0)]
    (if (= (type-definition-tag? tag) 1)
      (vector-push result decl)
      (if (= tag (ast-module-decl))
        (extract-type-definitions-module-body decl result 0 (vector-get decl 2))
        result))))

(defn extract-type-definitions-module-body [decl result idx count]
  (if (>= idx count)
    result
    (let [inner-decl (vector-get decl (+ idx 3))
      next-result (extract-type-definitions-decl inner-decl result)]
      (extract-type-definitions-module-body decl next-result (+ idx 1) count))))

;; typed defn の optional signature tag を検出する
(defn doc-defn-signature-node? [candidate]
  (if (= candidate 0)
    0
    (if (= (vector-get candidate 0) (ast-defn-signature)) 1 0)))

;; defn ノード末尾の metadata vector [doc-string, example-string, params, returns, invariant-expr] を参照する
(defn extract-defn-metadata [decl]
  (let [body-end (+ 4 (vector-get decl 2))
    meta-idx
      (if (< body-end (vector-length decl))
        (if (= (doc-defn-signature-node? (vector-get decl body-end)) 1)
          (+ body-end 1)
          body-end)
        (vector-length decl))]
    (if (< meta-idx (vector-length decl))
      (vector-get decl meta-idx)
      0)))

;; :doc メタデータからドキュメント文字列を抽出
(defn extract-doc-metadata [decl]
  (let [meta (extract-defn-metadata decl)]
    (if (= meta 0)
      ""
      (if (> (vector-length meta) 0)
        (vector-get meta 0)
        ""))))

;; :example メタデータからコード例テキストを抽出
(defn extract-example-metadata [decl]
  (let [meta (extract-defn-metadata decl)]
    (if (= meta 0)
      ""
      (if (> (vector-length meta) 1)
        (vector-get meta 1)
        ""))))

;; :params メタデータから [name-hash, doc-string] vector を抽出
(defn extract-param-doc-metadata [decl]
  (let [meta (extract-defn-metadata decl)]
    (if (= meta 0)
      (vector-new 0)
      (if (> (vector-length meta) 2)
        (vector-get meta 2)
        (vector-new 0)))))

;; :returns メタデータから戻り値説明を抽出
(defn extract-return-doc-metadata [decl]
  (let [meta (extract-defn-metadata decl)]
    (if (= meta 0)
      ""
      (if (> (vector-length meta) 3)
        (vector-get meta 3)
        ""))))

;; === payload 組み立て ===

(defn find-module-hash [ast]
  (find-module-hash-loop ast 0 (vector-length ast)))

(defn find-module-hash-loop [ast idx count]
  (if (>= idx count)
    0
    (let [decl (vector-get ast idx)]
      (if (= (vector-get decl 0) (ast-module-decl))
        (vector-get decl 1)
        (find-module-hash-loop ast (+ idx 1) count)))))

(defn title-from-hash [module-hash]
  (if (= module-hash 0)
    "module-global"
    (string-concat "module-" (symbol-from-hash module-hash))))

(defn title-from-ast [ast]
  (title-from-hash (find-module-hash ast)))

(defn title-from-module-id [module-id]
  (string-concat "module-" (int-to-string module-id)))

(defn symbol-candidates []
  "zyxwvutsrqponmlkjihgfedcba_ZYXWVUTSRQPONMLKJIHGFEDCBA?>=</-+*&%!")

(defn symbol-from-hash-search [hash candidates idx len]
  (if (>= idx len) ""
    (let [code (string-char-at candidates idx)]
      (if (> code hash)
        (symbol-from-hash-search hash candidates (+ idx 1) len)
        (if (= (% (- hash code) 31) 0)
          (let [prefix-hash (/ (- hash code) 31)
            ch (substring candidates idx (+ idx 1))]
            (if (= prefix-hash 0)
              ch
              (let [prefix (symbol-from-hash-search prefix-hash candidates 0 len)]
                (if (> (string-length prefix) 0)
                  (string-concat prefix ch)
                  (symbol-from-hash-search hash candidates (+ idx 1) len)))))
          (symbol-from-hash-search hash candidates (+ idx 1) len))))))

(defn symbol-from-hash [hash]
  (let [candidates (symbol-candidates)
    result
    (if (> hash 0)
      (symbol-from-hash-search hash candidates 0 (string-length candidates))
      "")]
    (if (> (string-length result) 0)
      result
      (string-concat "h" (int-to-string hash)))))

(defn make-function-entry [decl]
  (let [name-hash (vector-get decl 1)
    doc-text (extract-doc-metadata decl)
    example-text (extract-example-metadata decl)
    entry (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push entry name-hash)
            (symbol-from-hash name-hash))
          (vector-get decl 2))
        doc-text)
      example-text)))

(defn extract-function-entries [ast]
  (let [functions (extract-public-functions ast)]
    (extract-function-entries-loop functions (vector-new 0) 0 (vector-length functions))))

(defn extract-function-entries-loop [functions result idx count]
  (if (>= idx count)
    result
    (extract-function-entries-loop
      functions
      (vector-push result (make-function-entry (vector-get functions idx)))
      (+ idx 1)
      count)))

(defn type-kind-string [tag]
  (if (= tag (ast-type-decl))
    "type"
    (if (= tag (ast-recorddef))
      "recorddef"
      (if (= tag (ast-typealias))
        "typealias"
        (if (= tag (ast-typeconstrained))
          "typeconstrained"
          "type")))))

(defn make-type-entry [decl]
  (let [name-hash (vector-get decl 1)
    entry (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push entry name-hash)
        (symbol-from-hash name-hash))
      (type-kind-string (vector-get decl 0)))))

(defn extract-type-entries [ast]
  (let [types (extract-type-definitions ast)]
    (extract-type-entries-loop types (vector-new 0) 0 (vector-length types))))

(defn extract-type-entries-loop [types result idx count]
  (if (>= idx count)
    result
    (extract-type-entries-loop
      types
      (vector-push result (make-type-entry (vector-get types idx)))
      (+ idx 1)
      count)))

;; body 概要文字列を生成する (HTML レンダリングは HtmlDoc へ委譲)
;; functions/types の件数に応じた概要テキストを返す
(defn doc-entry-name [entry]
  (vector-get entry 1))

(defn make-doc-body-summary [functions types]
  (let [fn-count (vector-length functions)
    type-count (vector-length types)
    base
    (string-concat "functions:"
      (string-concat (int-to-string fn-count)
        (string-concat ",types:" (int-to-string type-count))))]
    (if (> fn-count 0)
      (let [with-fn
        (string-concat base
          (string-concat ",first-fn:" (doc-entry-name (vector-get functions 0))))]
        (if (> type-count 0)
          (string-concat with-fn
            (string-concat ",first-type:" (doc-entry-name (vector-get types 0))))
          with-fn))
      (if (> type-count 0)
        (string-concat base
          (string-concat ",first-type:" (doc-entry-name (vector-get types 0))))
        base))))

;; === deterministic 出力保証 ===

;; entry の先頭 slot をソートキーとして使う
(defn doc-entry-key [entry]
  (vector-get entry 0))

;; src[from..to) を out へコピー
(defn sort-doc-copy [src from to out]
  (if (>= from to)
    out
    (sort-doc-copy src (+ from 1) to (vector-push out (vector-get src from)))))

;; sorted の idx 位置へ elem を挿入する
(defn sort-doc-insert [sorted elem elem-key idx]
  (if (= idx 0)
    (let [out (vector-new (+ (vector-length sorted) 1))
      out (vector-push out elem)]
      (sort-doc-copy sorted 0 (vector-length sorted) out))
    (let [prev (vector-get sorted (- idx 1))
      prev-key (doc-entry-key prev)]
      (if (< elem-key prev-key)
        (sort-doc-insert sorted elem elem-key (- idx 1))
        (let [out (vector-new (+ (vector-length sorted) 1))
          out (sort-doc-copy sorted 0 idx out)
          out (vector-push out elem)]
          (sort-doc-copy sorted idx (vector-length sorted) out))))))

;; entries の idx 番目以降を順に挿入する
(defn sort-doc-loop [entries sorted idx len]
  (if (>= idx len)
    sorted
    (let [elem (vector-get entries idx)
      elem-key (doc-entry-key elem)
      next-sorted (sort-doc-insert sorted elem elem-key (vector-length sorted))]
      (sort-doc-loop entries next-sorted (+ idx 1) len))))

;; ドキュメント要素をソートして決定的順序にする
(defn sort-doc-entries [entries]
  (let [len (vector-length entries)]
    (if (< len 2)
      entries
      (let [first (vector-get entries 0)
        initial (vector-push (vector-new 1) first)]
        (sort-doc-loop entries initial 1 len)))))

;; 環境依存情報のフィルタリング
(defn filter-env-dependent [doc]
  doc)

;; === HTML ドキュメント生成 ===

;; generate-html: AST から HTML ドキュメント構造を生成
;; 出力: [tag, title, body, functions, types]
(defn generate-html [ast opts]
  (let [functions (sort-doc-entries (extract-doc-function-entries ast))
    types (sort-doc-entries (extract-type-entries ast))
    title (title-from-ast ast)
    body (make-doc-body-summary functions types)
    doc (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push doc 1)
            title)
          body)
        functions)
      types)))

;; === スキーマ準拠出力 ===

;; knowledge 用の型名レンダリング
(defn knowledge-con-type-text [name-hash]
  (if (= name-hash 100)
    "Int"
    (if (= name-hash 200)
      "Bool"
      (if (= name-hash 300)
        "String"
        (if (= name-hash 400)
          "Float"
          (if (= name-hash 500)
            "Unit"
            (symbol-from-hash name-hash)))))))

(defn knowledge-unknown-type-text []
  "Unknown")

(defn knowledge-render-type-text [ty]
  (if (= ty 0)
    (knowledge-unknown-type-text)
    (let [tag (ty-tag ty)]
      (if (= tag (ty-con))
        (knowledge-con-type-text (ty-name ty))
        (if (= tag (ty-var))
          (string-concat "t" (int-to-string (ty-name ty)))
          (if (= tag (ty-fun))
            "Fn"
            (if (= tag 4)
              (string-concat "record-" (symbol-from-hash (ty-name ty)))
              (knowledge-unknown-type-text))))))))

(defn knowledge-param-string [name-hash ty]
  (string-concat
    (symbol-from-hash name-hash)
    (string-concat ":" (knowledge-render-type-text ty))))

(defn knowledge-build-param-entries-loop [decl fun-ty idx count params]
  (if (>= idx count)
    params
    (let [name-hash (vector-get decl (+ 3 idx))
      param-ty (if (= (ty-tag fun-ty) (ty-fun)) (ty-fp fun-ty) 0)
      rest-ty (if (= (ty-tag fun-ty) (ty-fun)) (ty-fr fun-ty) 0)
      next-params (vector-push params (knowledge-param-string name-hash param-ty))]
      (knowledge-build-param-entries-loop decl rest-ty (+ idx 1) count next-params))))

(defn knowledge-build-param-entries [decl fun-ty]
  (let [count (vector-get decl 2)]
    (knowledge-build-param-entries-loop decl fun-ty 0 count (vector-new count))))

(defn find-param-doc-text-loop [param-metadata name-hash idx count]
  (if (>= idx count)
    ""
    (let [entry (vector-get param-metadata idx)]
      (if (= (vector-get entry 0) name-hash)
        (vector-get entry 1)
        (find-param-doc-text-loop param-metadata name-hash (+ idx 1) count)))))

(defn find-param-doc-text [param-metadata name-hash]
  (find-param-doc-text-loop param-metadata name-hash 0 (vector-length param-metadata)))

(defn make-doc-function-param-entry [name-hash ty param-metadata]
  (let [entry (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push entry (symbol-from-hash name-hash))
        (knowledge-render-type-text ty))
      (find-param-doc-text param-metadata name-hash))))

(defn doc-build-param-entries-loop [decl fun-ty param-metadata idx count params]
  (if (>= idx count)
    params
    (let [name-hash (vector-get decl (+ 3 idx))
      param-ty (if (= (ty-tag fun-ty) (ty-fun)) (ty-fp fun-ty) 0)
      rest-ty (if (= (ty-tag fun-ty) (ty-fun)) (ty-fr fun-ty) 0)
      next-params
      (vector-push params (make-doc-function-param-entry name-hash param-ty param-metadata))]
      (doc-build-param-entries-loop decl rest-ty param-metadata (+ idx 1) count next-params))))

(defn doc-build-param-entries [decl fun-ty]
  (let [count (vector-get decl 2)
    param-metadata (extract-param-doc-metadata decl)]
    (doc-build-param-entries-loop decl fun-ty param-metadata 0 count (vector-new count))))

(defn knowledge-drop-arity-loop [ty remaining]
  (if (<= remaining 0)
    ty
    (if (= ty 0)
      0
      (if (= (ty-tag ty) (ty-fun))
        (knowledge-drop-arity-loop (ty-fr ty) (- remaining 1))
        0))))

(defn knowledge-return-type-text [decl fun-ty]
  (knowledge-render-type-text
    (knowledge-drop-arity-loop fun-ty (vector-get decl 2))))

(defn make-doc-function-return-entry [decl fun-ty]
  (let [entry (vector-new 2)]
    (vector-push
      (vector-push entry (knowledge-return-type-text decl fun-ty))
      (extract-return-doc-metadata decl))))

(defn make-knowledge-function-entry [decl fun-ty]
  (let [name-hash (vector-get decl 1)
    params (knowledge-build-param-entries decl fun-ty)
    returns (knowledge-return-type-text decl fun-ty)
    doc-text (extract-doc-metadata decl)
    example-text (extract-example-metadata decl)
    entry (vector-new 7)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push entry name-hash)
                (symbol-from-hash name-hash))
              (vector-get decl 2))
            params)
          returns)
        doc-text)
      example-text)))

(defn make-doc-function-entry [decl fun-ty]
  (let [name-hash (vector-get decl 1)
    params (doc-build-param-entries decl fun-ty)
    returns (make-doc-function-return-entry decl fun-ty)
    doc-text (extract-doc-metadata decl)
    example-text (extract-example-metadata decl)
    entry (vector-new 7)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push entry name-hash)
                (symbol-from-hash name-hash))
              (vector-get decl 2))
            params)
          returns)
        doc-text)
      example-text)))

(defn make-knowledge-state [entries env]
  (vector-push
    (vector-push (vector-new 2) entries)
    env))

(defn knowledge-state-entries [state]
  (vector-get state 0))

(defn knowledge-state-env [state]
  (vector-get state 1))

(defn knowledge-result-type [out]
  (if (= (result-failed out) 1)
    0
    (result-type out)))

(defn knowledge-next-env [out env]
  (if (> (vector-length out) 3)
    (vector-get out 3)
    env))

(defn collect-knowledge-function-range [decls base idx count entries env counter emit-entry]
  (if (>= idx count)
    (make-knowledge-state entries env)
    (let [decl (vector-get decls (+ base idx))
      state (collect-knowledge-function-decl decl entries env counter emit-entry)]
      (do
        (root_push state)
        (let [next-entries (knowledge-state-entries state)
          next-env (knowledge-state-env state)]
          (do
            (root_push next-entries)
            (root_push next-env)
            (let [next-state
              (collect-knowledge-function-range
                decls
                base
                (+ idx 1)
                count
                next-entries
                next-env
                counter
                emit-entry)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                next-state))))))))

(defn collect-knowledge-function-module-body [decl entries env counter emit-entry]
  (collect-knowledge-function-range
    decl
    3
    0
    (vector-get decl 2)
    entries
    env
    counter
    emit-entry))

(defn collect-knowledge-function-defn [decl entries env counter emit-entry]
  (let [out (infer-defn decl env counter)
    next-env (knowledge-next-env out env)
    next-entries
    (if (= emit-entry 1)
      (vector-push entries (make-knowledge-function-entry decl (knowledge-result-type out)))
      entries)]
    (make-knowledge-state next-entries next-env)))

(defn collect-knowledge-function-decl [decl entries env counter emit-entry]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (collect-knowledge-function-defn decl entries env counter emit-entry)
      (if (= tag (ast-module-decl))
        (collect-knowledge-function-module-body decl entries env counter emit-entry)
        (if (= tag (ast-private))
          (collect-knowledge-function-decl (vector-get decl 1) entries env counter 0)
          (make-knowledge-state entries env))))))

(defn collect-doc-function-range [decls base idx count entries env counter emit-entry]
  (if (>= idx count)
    (make-knowledge-state entries env)
    (let [decl (vector-get decls (+ base idx))
      state (collect-doc-function-decl decl entries env counter emit-entry)]
      (do
        (root_push state)
        (let [next-entries (knowledge-state-entries state)
          next-env (knowledge-state-env state)]
          (do
            (root_push next-entries)
            (root_push next-env)
            (let [next-state
              (collect-doc-function-range
                decls
                base
                (+ idx 1)
                count
                next-entries
                next-env
                counter
                emit-entry)]
              (do
                (root_pop)
                (root_pop)
                (root_pop)
                next-state))))))))

(defn collect-doc-function-module-body [decl entries env counter emit-entry]
  (collect-doc-function-range
    decl
    3
    0
    (vector-get decl 2)
    entries
    env
    counter
    emit-entry))

(defn collect-doc-function-defn [decl entries env counter emit-entry]
  (let [out (infer-defn decl env counter)
    next-env (knowledge-next-env out env)
    next-entries
    (if (= emit-entry 1)
      (vector-push entries (make-doc-function-entry decl (knowledge-result-type out)))
      entries)]
    (make-knowledge-state next-entries next-env)))

(defn collect-doc-function-decl [decl entries env counter emit-entry]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (collect-doc-function-defn decl entries env counter emit-entry)
      (if (= tag (ast-module-decl))
        (collect-doc-function-module-body decl entries env counter emit-entry)
        (if (= tag (ast-private))
          (collect-doc-function-decl (vector-get decl 1) entries env counter 0)
          (make-knowledge-state entries env))))))

(defn extract-knowledge-function-entries [ast]
  (let [counter (make-var-counter)
    env (init-builtin-env counter)
    state (collect-knowledge-function-range ast 0 0 (vector-length ast) (vector-new 0) env counter 1)]
    (knowledge-state-entries state)))

(defn extract-doc-function-entries [ast]
  (let [counter (make-var-counter)
    env (init-builtin-env counter)
    state (collect-doc-function-range ast 0 0 (vector-length ast) (vector-new 0) env counter 1)]
    (knowledge-state-entries state)))

;; generate-knowledge: [module-id, functions, types]
(defn generate-knowledge [ast module-id]
  (let [functions (sort-doc-entries (extract-knowledge-function-entries ast))
    types (sort-doc-entries (extract-type-entries ast))
    doc (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push doc module-id)
        functions)
      types)))

;; === review payload ===

;; review diagnostic: [rule-id, title, body, severity, line, column, code]
(defn make-review-diagnostic [rule-id title body severity line column code]
  (let [d (vector-new 7)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push
              (vector-push
                (vector-push d rule-id)
                title)
              body)
            severity)
          line)
        column)
      code)))

(defn review-warning-severity []
  "warning")

(defn review-diagnostics-new []
  (vector-new 8))

(defn review-add [results diagnostic]
  (if (= diagnostic 0)
    results
    (vector-push results diagnostic)))

(defn review-unused-let-diagnostic [node]
  (if (= (vector-get node 0) (ast-let))
    (let [name-hash (vector-get node 1)
      body (vector-get node 3)]
      (if (= (ast-contains-var body name-hash) 0)
        (make-review-diagnostic
          100
          "unused-let"
          (string-concat "let binding " (string-concat (symbol-from-hash name-hash) " is not used"))
          (review-warning-severity)
          1
          1
          "L0001")
        0))
    0))

(defn review-empty-do-diagnostic [node]
  (if (= (vector-get node 0) (ast-do))
    (if (= (vector-get node 1) 0)
      (make-review-diagnostic 104 "empty-do" "do block has no expressions" (review-warning-severity) 1 1 "L0002")
      0)
    0))

(defn review-collect-recordlit-loop [node results idx count]
  (if (>= idx count)
    results
    (review-collect-recordlit-loop
      node
      (review-collect-node (vector-get node (+ 4 (* idx 2))) results)
      (+ idx 1)
      count)))

(defn review-collect-recordupdate-loop [node results idx count]
  (if (>= idx count)
    results
    (review-collect-recordupdate-loop
      node
      (review-collect-node (vector-get node (+ 4 (* idx 2))) results)
      (+ idx 1)
      count)))

(defn review-collect-computation-loop [node results idx count]
  (if (>= idx count)
    results
    (review-collect-computation-loop
      node
      (review-collect-node (vector-get node (+ 5 (* idx 3))) results)
      (+ idx 1)
      count)))

(defn review-collect-apply-loop [node results idx count]
  (if (>= idx count)
    results
    (review-collect-apply-loop
      node
      (review-collect-node (vector-get node (+ 3 idx)) results)
      (+ idx 1)
      count)))

(defn review-collect-do-loop [node results idx count]
  (if (>= idx count)
    results
    (review-collect-do-loop
      node
      (review-collect-node (vector-get node (+ 2 idx)) results)
      (+ idx 1)
      count)))

(defn review-collect-match-loop [node results idx count]
  (if (>= idx count)
    results
    (review-collect-match-loop
      node
      (review-collect-node (vector-get node (+ 4 (* idx 2))) results)
      (+ idx 1)
      count)))

(defn review-collect-node [node results]
  (let [with-unused (review-add results (review-unused-let-diagnostic node))
    with-rules (review-add with-unused (review-empty-do-diagnostic node))
    tag (vector-get node 0)]
    (if (= tag (ast-ann))
      (review-collect-node (vector-get node 1) with-rules)
      (if (= tag (ast-recordlit))
        (review-collect-recordlit-loop node with-rules 0 (vector-get node 2))
        (if (= tag (ast-fieldaccess))
          (review-collect-node (vector-get node 1) with-rules)
          (if (= tag (ast-recordupdate))
            (let [with-base (review-collect-node (vector-get node 1) with-rules)]
              (review-collect-recordupdate-loop node with-base 0 (vector-get node 2)))
            (if (= tag (ast-computation))
              (review-collect-computation-loop node with-rules 0 (vector-get node 2))
              (if (= tag (ast-quote))
                (review-collect-node (vector-get node 1) with-rules)
                (if (= tag (ast-unquote))
                  (review-collect-node (vector-get node 1) with-rules)
                  (if (= tag (ast-unquote-splice))
                    (review-collect-node (vector-get node 1) with-rules)
                    (if (= tag (ast-if))
                      (let [with-cond (review-collect-node (vector-get node 1) with-rules)
                        with-then (review-collect-node (vector-get node 2) with-cond)]
                        (review-collect-node (vector-get node 3) with-then))
                      (if (= tag (ast-let))
                        (let [with-init (review-collect-node (vector-get node 2) with-rules)]
                          (review-collect-node (vector-get node 3) with-init))
                        (if (= tag (ast-apply))
                          (review-collect-apply-loop node with-rules 0 (vector-get node 2))
                          (if (= tag (ast-lambda))
                            (review-collect-node (vector-get node (+ 2 (vector-get node 1))) with-rules)
                            (if (= tag (ast-do))
                              (review-collect-do-loop node with-rules 0 (vector-get node 1))
                              (if (= tag (ast-match))
                                (let [with-scrutinee (review-collect-node (vector-get node 1) with-rules)]
                                  (review-collect-match-loop node with-scrutinee 0 (vector-get node 2)))
                                with-rules))))))))))))))))

(defn review-defn-body [decl]
  (vector-get decl (+ 3 (vector-get decl 2))))

(defn review-functions-loop [functions results idx count]
  (if (>= idx count)
    results
    (review-functions-loop
      functions
      (review-collect-node (review-defn-body (vector-get functions idx)) results)
      (+ idx 1)
      count)))

;; generate-review: [source-id, diagnostics]
(defn generate-review [ast source-id]
  (let [functions (extract-public-functions ast)
    diagnostics (review-functions-loop functions (review-diagnostics-new) 0 (vector-length functions))
    doc (vector-new 2)]
    (vector-push
      (vector-push doc source-id)
      diagnostics)))

(defn review-summary-title [diagnostics]
  (if (> (vector-length diagnostics) 0)
    (vector-get (vector-get diagnostics 0) 1)
    "clean"))

(defn review-summary-body [diagnostics]
  (if (> (vector-length diagnostics) 0)
    (let [diag (vector-get diagnostics 0)]
      (string-concat
        "diagnostics:"
        (string-concat
          (int-to-string (vector-length diagnostics))
          (string-concat ",first-body:" (vector-get diag 2)))))
    "diagnostics:0"))

(defn review-summary-severity [diagnostics]
  (if (> (vector-length diagnostics) 0)
    (vector-get (vector-get diagnostics 0) 3)
    "clean"))

(defn review-summary-code-location [diagnostics]
  (if (> (vector-length diagnostics) 0)
    (let [diag (vector-get diagnostics 0)
      code (vector-get diag 6)
      line (vector-get diag 4)
      column (vector-get diag 5)]
      (string-concat
        code
        (string-concat
          "@"
          (string-concat
            (int-to-string line)
            (string-concat ":" (int-to-string column))))))
    "-"))

;; doc-output の HTML section metadata [id, count] を組み立てる
(defn make-doc-section [section-id count]
  (vector-push
    (vector-push (vector-new 2) section-id)
    count))

(defn append-doc-section [sections section-id count]
  (if (> count 0)
    (vector-push sections (make-doc-section section-id count))
    sections))

(defn build-doc-sections [functions types]
  (let [sections0 (vector-new 2)
    sections1 (append-doc-section sections0 "functions" (vector-length functions))
    sections2 (append-doc-section sections1 "types" (vector-length types))]
    sections2))

;; generate-doc-output: [module-id, functions, types, html-title, html-sections]
(defn generate-doc-output [ast module-id]
  (let [functions (sort-doc-entries (extract-doc-function-entries ast))
    types (sort-doc-entries (extract-type-entries ast))
    sections (build-doc-sections functions types)
    module-hash (find-module-hash ast)
    title (if (= module-hash 0) (title-from-module-id module-id) (title-from-hash module-hash))
    doc (vector-new 5)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push doc module-id)
            functions)
          types)
        title)
      sections)))

;; === doc-ack/doc-check payload ===

(defn doc-ack-status-text []
  "ack:recorded")

(defn doc-check-status-text []
  "status:ok")

(defn doc-trailer-comment-prefix []
  "; ")

(defn doc-reviewed-by-prefix []
  "; Doc-Reviewed-By: ")

(defn doc-review-status-prefix []
  "; Doc-Review-Status: ")

(defn doc-reviewed-by-line [reviewer]
  (string-concat (doc-reviewed-by-prefix) reviewer))

(defn doc-review-status-line [status]
  (string-concat (doc-review-status-prefix) status))

(defn doc-trailer-has-prefix? [line prefix]
  (let [prefix-len (string-length prefix)
    line-len (string-length line)]
    (if (< line-len prefix-len)
      0
      (if (string-eq (substring line 0 prefix-len) prefix)
        1
        0))))

(defn doc-trailer-value [line prefix]
  (let [prefix-len (string-length prefix)
    line-len (string-length line)]
    (if (= (doc-trailer-has-prefix? line prefix) 1)
      (substring line prefix-len line-len)
      "")))

(defn doc-trim-trailing-line-endings-loop [src end]
  (if (< end 1)
    0
    (let [ch (string-char-at src (- end 1))]
      (if (or (= ch 10) (= ch 13))
        (doc-trim-trailing-line-endings-loop src (- end 1))
        end))))

(defn doc-find-line-start-loop [src idx]
  (if (< idx 1)
    0
    (if (= (string-char-at src (- idx 1)) 10)
      idx
      (doc-find-line-start-loop src (- idx 1)))))

(defn doc-last-line-from-end [src end]
  (if (< end 1)
    ""
    (let [start (doc-find-line-start-loop src end)]
      (substring src start end))))

(defn doc-last-nonempty-line [src]
  (let [end (doc-trim-trailing-line-endings-loop src (string-length src))]
    (doc-last-line-from-end src end)))

(defn doc-second-last-nonempty-line [src]
  (let [end-2 (doc-trim-trailing-line-endings-loop src (string-length src))]
    (if (< end-2 1)
      ""
      (let [start-2 (doc-find-line-start-loop src end-2)
        end-1 (doc-trim-trailing-line-endings-loop src start-2)]
        (doc-last-line-from-end src end-1)))))

(defn doc-reviewed-by-line-valid? [line]
  (if (> (string-length (doc-trailer-value line (doc-reviewed-by-prefix))) 0) 1 0))

(defn doc-review-status-line-valid? [line]
  (let [status (doc-trailer-value line (doc-review-status-prefix))]
    (if (string-eq status "Passed")
      1
      (if (string-eq status "Failed") 1 0))))

(defn doc-check-trailer-valid? [src]
  (let [reviewer-line (doc-last-nonempty-line src)
    status-line (doc-second-last-nonempty-line src)]
    (if (= (doc-review-status-line-valid? status-line) 1)
      (doc-reviewed-by-line-valid? reviewer-line)
      0)))

(defn generate-doc-ack [ast reviewer]
  (let [doc (generate ast 0)
    trailers (vector-push (vector-new 1) (doc-reviewed-by-line reviewer))
    result (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push result (doc-ack-status-text))
          (vector-get doc 0))
        (vector-get doc 1))
      trailers)))

(defn generate-doc-check [ast reviewer]
  (let [doc (generate ast 0)
    trailers
    (vector-push
      (vector-push (vector-new 1) (doc-review-status-line "Passed"))
      (doc-reviewed-by-line reviewer))
    result (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push result (doc-check-status-text))
          (vector-get doc 0))
        (vector-get doc 1))
      trailers)))

;; 検証用 main
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
    doc (generate program 0)]
    (do
      (print (vector-length doc))
      (print (vector-length (vector-get doc 2)))
      0)))
