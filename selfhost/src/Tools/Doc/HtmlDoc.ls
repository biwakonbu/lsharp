(module Tools.Doc.HtmlDoc)
(import Tools.Doc.DocTools)
(import Tools.Doc.HtmlTemplate)
(import Tools.Doc.HtmlLayout)

;; HtmlDoc.ls - L# 製 HTML ドキュメント生成
;;
;; DocTools.ls が返す決定的 payload を HTML 文字列へ変換する。
;; HtmlTemplate の DSL と HtmlLayout のレイアウトを使用する。

;; === 関数エントリ → HTML 文字列 ===

;; 関数エントリ [hash, name, arity] を "<li>{name}/{arity}</li>" に変換する
(defn render-function-signature [func-doc]
  (render-node
    (elem "li" (vector-new 0)
      (vector-push (vector-new 1)
        (raw-node (string-concat (vector-get func-doc 1)
            (string-concat "/" (int-to-string (vector-get func-doc 2)))))))))

;; 型エントリ [hash, name, kind] を "<li>{kind} {name}</li>" に変換する
(defn render-type-definition [type-doc]
  (render-node
    (elem "li" (vector-new 0)
      (vector-push (vector-new 1)
        (raw-node (string-concat (vector-get type-doc 2)
            (string-concat " " (vector-get type-doc 1))))))))

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

;; === ガイド / サイトインデックス ===

;; リンク先とラベルから <li><a href="...">...</a></li> を組み立てる
(defn render-link-item [href label]
  (string-concat "<li><a href=\""
    (string-concat (html-escape href)
      (string-concat "\">"
        (string-concat (html-escape label) "</a></li>")))))

;; guide-link [href, label] の vector から一覧 HTML を生成する
(defn render-guide-items-loop [guides idx count]
  (if (>= idx count)
    ""
    (let [guide-link (vector-get guides idx)
      href (vector-get guide-link 0)
      label (vector-get guide-link 1)]
      (string-concat
        (render-link-item href label)
        (render-guide-items-loop guides (+ idx 1) count)))))

;; module 名 vector から /api/{module}.html への一覧 HTML を生成する
(defn render-module-link-items-loop [modules idx count]
  (if (>= idx count)
    ""
    (let [module-name (vector-get modules idx)
      href (string-concat "api/"
        (string-concat module-name ".html"))]
      (string-concat
        (render-link-item href module-name)
        (render-module-link-items-loop modules (+ idx 1) count)))))

;; 単一 guide ページを完全な HTML ドキュメントへ変換する
(defn render-guide-page [title content-html]
  (guide-page-layout title content-html))

;; guides と modules をまとめた doc site index を生成する
(defn render-doc-site-index [guides modules]
  (doc-site-index-layout
    (render-guide-items-loop guides 0 (vector-length guides))
    (render-module-link-items-loop modules 0 (vector-length modules))))

;; 検証用 main
(defn main []
  (let [doc (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push (vector-new 5) 0)
            "module-global")
          "<p>content</p>")
        (vector-new 0))
      (vector-new 0))]
    (do
      (print (string-length (render-html doc 0)))
      0)))
