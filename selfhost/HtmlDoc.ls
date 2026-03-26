(module HtmlDoc)
(import DocTools)
(import HtmlTemplate)
(import HtmlLayout)

;; HtmlDoc.ls - L# 製 HTML ドキュメント生成
;;
;; DocTools.ls が返す決定的 payload を HTML 文字列へ変換する。
;; HtmlTemplate の DSL と HtmlLayout のレイアウトを使用する。

;; === 関数エントリ → HTML 文字列 ===

;; 関数エントリ [id, arity] を "<li>fn-{id}/{arity}</li>" に変換する
(defn render-function-signature [func-doc]
  (render-node
    (elem "li" (vector-new 0)
      (vector-push (vector-new 1)
        (raw-node (string-concat "fn-"
          (string-concat (int-to-string (vector-get func-doc 0))
            (string-concat "/" (int-to-string (vector-get func-doc 1))))))))))

;; 型エントリ [id, kind] を "<li>{kind}-{id}</li>" に変換する
(defn render-type-definition [type-doc]
  (render-node
    (elem "li" (vector-new 0)
      (vector-push (vector-new 1)
        (raw-node (string-concat (vector-get type-doc 1)
          (string-concat "-" (int-to-string (vector-get type-doc 0)))))))))

;; === リスト項目ループ ===

(defn render-function-items-loop [functions idx count]
  (if (>= idx count)
    ""
    (string-concat
      (render-function-signature (vector-get functions idx))
      (render-function-items-loop functions (+ idx 1) count))))

(defn render-type-items-loop [types idx count]
  (if (>= idx count)
    ""
    (string-concat
      (render-type-definition (vector-get types idx))
      (render-type-items-loop types (+ idx 1) count))))

;; === セクション生成 ===

;; 関数セクション HTML を生成する
(defn render-functions-section-html [functions]
  (if (= (vector-length functions) 0)
    ""
    (string-concat
      "<section id=\"functions\"><ul>"
      (string-concat
        (render-function-items-loop functions 0 (vector-length functions))
        "</ul></section>"))))

;; 型セクション HTML を生成する
(defn render-types-section-html [types]
  (if (= (vector-length types) 0)
    ""
    (string-concat
      "<section id=\"types\"><ul>"
      (string-concat
        (render-type-items-loop types 0 (vector-length types))
        "</ul></section>"))))

;; === モジュールページ ===

;; module-doc [tag, title, body, functions, types] からモジュールページを生成する
(defn render-module-page [module-doc]
  (let [title (vector-get module-doc 1)
        functions (vector-get module-doc 3)
        types (vector-get module-doc 4)
        functions-section (render-functions-section-html functions)
        types-section (render-types-section-html types)
        body
          (if (> (string-length functions-section) 0)
            (string-concat functions-section types-section)
            (if (> (string-length types-section) 0)
              types-section
              (vector-get module-doc 2)))]
    (string-concat "<main><h1>"
      (string-concat title
        (string-concat "</h1>"
          (string-concat body "</main>"))))))

;; === HTML ドキュメント生成 ===

;; doc payload + opts から完全な HTML ドキュメントを生成する
(defn render-html [doc opts]
  (let [title (vector-get doc 1)
        page-content (render-module-page doc)]
    (base-layout title page-content)))

;; === インデックスページ ===

(defn render-index-items-loop [modules idx count]
  (if (>= idx count)
    ""
    (string-concat
      (string-concat "<li>" (string-concat (vector-get modules idx) "</li>"))
      (render-index-items-loop modules (+ idx 1) count))))

;; モジュール一覧インデックスページを生成する
(defn render-index [modules]
  (index-page-layout
    (render-index-items-loop modules 0 (vector-length modules))))

;; 検証用 main
(defn main []
  (let [doc (generate-html (parse-program "(defn main [] 42)") 0)]
    (do
      (print (string-length (render-html doc 0)))
      0)))
