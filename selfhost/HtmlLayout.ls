(module HtmlLayout)
(import HtmlTemplate)

;; HtmlLayout.ls - L# 製 HTML レイアウトテンプレート
;;
;; HtmlTemplate のプリミティブを使い、共通 HTML レイアウトを定義する。
;; DocTools / HtmlDoc から利用される。

;; === CSS ===

;; モジュールドキュメント用の最小 CSS (外部ファイル依存なし)
(defn css-inline []
  "body{font-family:sans-serif;max-width:48rem;margin:0 auto;padding:1rem}h1{border-bottom:1px solid #ccc}ul{list-style:none;padding:0}li{padding:0.25rem 0}")

;; === 共通レイアウト ===

;; 完全な HTML ドキュメントを生成する
;; title: ページタイトル (エスケープされる)
;; content-html: body 内に挿入する HTML 文字列
(defn base-layout [title content-html]
  (string-concat (doctype)
    (string-concat "<html><head><meta charset=\"utf-8\"><title>"
      (string-concat (html-escape title)
        (string-concat "</title><style>"
          (string-concat (css-inline)
            (string-concat "</style></head><body>"
              (string-concat content-html "</body></html>"))))))))

;; === ドキュメントページレイアウト ===

;; モジュールドキュメント用レイアウト
;; title: モジュール名
;; functions-html: 関数セクション HTML
;; types-html: 型セクション HTML
(defn doc-page-layout [title functions-html types-html]
  (base-layout title
    (string-concat "<main><h1>"
      (string-concat (html-escape title)
        (string-concat "</h1>"
          (string-concat functions-html
            (string-concat types-html "</main>")))))))

;; === インデックスページレイアウト ===

;; モジュール一覧インデックスページ用レイアウト
;; modules-html: モジュール一覧の <li> タグ群
(defn index-page-layout [modules-html]
  (base-layout "modules"
    (string-concat "<main><h1>modules</h1><ul>"
      (string-concat modules-html "</ul></main>"))))

;; ガイドページ用レイアウト
;; title: ページタイトル
;; content-html: Markdown から変換済みの本文 HTML
(defn guide-page-layout [title content-html]
  (base-layout title
    (string-concat "<main class=\"guide\">"
      (string-concat content-html "</main>"))))

;; ドキュメントサイトのトップページ用レイアウト
;; guides-html: ガイド一覧の <li> タグ群
;; modules-html: API モジュール一覧の <li> タグ群
(defn doc-site-index-layout [guides-html modules-html]
  (base-layout "L# Documentation"
    (string-concat "<main><h1>L# Documentation</h1>"
      (string-concat "<section id=\"guides\"><h2>Guides</h2><ul>"
        (string-concat guides-html
          (string-concat "</ul></section><section id=\"api\"><h2>Modules</h2><ul>"
            (string-concat modules-html "</ul></section></main>")))))))

;; 検証用 main
(defn main []
  (let [html (base-layout "Test" "<p>content</p>")]
    (do
      (print (string-length html))
      0)))
