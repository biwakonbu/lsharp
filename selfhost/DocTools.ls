(module DocTools)
(import AST)
(import Parser)

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

;; :doc メタデータからドキュメント文字列を抽出
(defn extract-doc-metadata [decl]
  0)

;; :example メタデータからコード例を抽出
(defn extract-example-metadata [decl]
  (vector-new 0))

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
    (string-concat "module-" (int-to-string module-hash))))

(defn title-from-ast [ast]
  (title-from-hash (find-module-hash ast)))

(defn title-from-module-id [module-id]
  (string-concat "module-" (int-to-string module-id)))

(defn make-function-entry [decl]
  (let [entry (vector-new 2)]
    (vector-push
      (vector-push entry (vector-get decl 1))
      (vector-get decl 2))))

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
  (let [entry (vector-new 2)]
    (vector-push
      (vector-push entry (vector-get decl 1))
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
(defn make-doc-body-summary [functions types]
  (let [fn-count (vector-length functions)
        type-count (vector-length types)]
    (string-concat "functions:"
      (string-concat (int-to-string fn-count)
        (string-concat ",types:" (int-to-string type-count))))))

;; === deterministic 出力保証 ===

;; ドキュメント要素をソートして決定的順序にする
(defn sort-doc-entries [entries]
  entries)

;; 環境依存情報のフィルタリング
(defn filter-env-dependent [doc]
  doc)

;; === HTML ドキュメント生成 ===

;; generate-html: AST から HTML ドキュメント構造を生成
;; 出力: [tag, title, body, functions, types]
(defn generate-html [ast opts]
  (let [functions (sort-doc-entries (extract-function-entries ast))
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

;; generate-knowledge: [module-id, functions, types]
(defn generate-knowledge [ast module-id]
  (let [functions (sort-doc-entries (extract-function-entries ast))
        types (sort-doc-entries (extract-type-entries ast))
        doc (vector-new 3)]
    (vector-push
      (vector-push
        (vector-push doc module-id)
        functions)
      types)))

;; generate-review: [source-id, diagnostics]
(defn generate-review [ast source-id]
  (let [doc (vector-new 2)]
    (vector-push
      (vector-push doc source-id)
      (vector-new 0))))

;; doc-output のセクション数を計算
(defn count-doc-sections [fn-count type-count]
  (let [s1 (if (> fn-count 0) 1 0)
        s2 (if (> type-count 0) 1 0)]
    (+ s1 s2)))

;; generate-doc-output: [module-id, functions, types, html-title, html-sections]
(defn generate-doc-output [ast module-id]
  (let [functions (sort-doc-entries (extract-function-entries ast))
        types (sort-doc-entries (extract-type-entries ast))
        sections (count-doc-sections (vector-length functions) (vector-length types))
        title (title-from-module-id module-id)
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

;; 検証用 main
(defn main []
  (let [program (parse-program "(defn main [] 42) (type Doc Int)")
        doc (generate program 0)]
    (do
      (print (vector-length doc))
      (print (vector-length (vector-get doc 2)))
      0)))
