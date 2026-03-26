(module HtmlTemplate)

;; HtmlTemplate.ls - L# 製 HTML テンプレートエンジン
;;
;; S 式 DSL でテンプレートノードを組み立て、HTML 文字列へレンダリングする。
;; html-escape による XSS 安全なエスケープを提供する。

;; === HTML エスケープ ===

;; 特殊文字のバイト値に対応するエンティティ文字列を返す
(defn html-escape-char [code]
  (if (= code 38) "&amp;"
    (if (= code 60) "&lt;"
      (if (= code 62) "&gt;"
        (if (= code 34) "&quot;"
          (if (= code 39) "&#39;"
            ""))))))

;; バイト値が HTML 特殊文字か判定する (1=要エスケープ, 0=不要)
(defn needs-escape? [code]
  (if (= code 38) 1
    (if (= code 60) 1
      (if (= code 62) 1
        (if (= code 34) 1
          (if (= code 39) 1 0))))))

;; 文字列を 1 バイトずつ走査してエスケープ済み文字列を構築する
(defn html-escape-loop [s idx len result]
  (if (>= idx len)
    result
    (let [code (string-char-at s idx)]
      (if (= (needs-escape? code) 1)
        (html-escape-loop s (+ idx 1) len
          (string-concat result (html-escape-char code)))
        (html-escape-loop s (+ idx 1) len
          (string-concat result (substring s idx (+ idx 1))))))))

;; 文字列中の <>&"' を HTML エンティティに変換する
(defn html-escape [s]
  (html-escape-loop s 0 (string-length s) ""))

;; === ノード表現 ===
;; element: [1, tag-name, attrs-vec, children-vec]
;; text:    [2, escaped-string]
;; raw:     [3, html-string]

;; 要素ノードを生成する
(defn elem [tag attrs children]
  (let [node (vector-new 4)]
    (vector-push
      (vector-push
        (vector-push
          (vector-push node 1)
          tag)
        attrs)
      children)))

;; エスケープ済みテキストノードを生成する
(defn text [value]
  (let [node (vector-new 2)]
    (vector-push
      (vector-push node 2)
      (html-escape value))))

;; エスケープなし raw HTML ノードを生成する
(defn raw-node [value]
  (let [node (vector-new 2)]
    (vector-push
      (vector-push node 3)
      value)))

;; === void element 判定 ===

;; 閉じタグが不要な HTML 要素か判定する (1=void, 0=通常)
(defn void-element? [tag]
  (if (string-eq tag "br") 1
    (if (string-eq tag "hr") 1
      (if (string-eq tag "img") 1
        (if (string-eq tag "input") 1
          (if (string-eq tag "meta") 1
            (if (string-eq tag "link") 1 0)))))))

;; === 属性レンダリング ===

;; 単一属性を ` key="escaped-value"` 形式に変換する
(defn render-attr [attr]
  (string-concat " "
    (string-concat (vector-get attr 0)
      (string-concat "=\""
        (string-concat (html-escape (vector-get attr 1)) "\"")))))

;; attrs vector をループして属性文字列を連結する
(defn render-attrs-loop [attrs idx len result]
  (if (>= idx len)
    result
    (render-attrs-loop attrs (+ idx 1) len
      (string-concat result (render-attr (vector-get attrs idx))))))

;; 全属性を文字列にレンダリングする
(defn render-attrs [attrs]
  (render-attrs-loop attrs 0 (vector-length attrs) ""))

;; === ノードレンダリング ===

;; children vector をループしてレンダリング結果を連結する
(defn render-children-loop [children idx len result]
  (if (>= idx len)
    result
    (render-children-loop children (+ idx 1) len
      (string-concat result (render-node (vector-get children idx))))))

;; テンプレートノードを HTML 文字列に変換する (再帰)
(defn render-node [node]
  (let [tag-id (vector-get node 0)]
    (if (= tag-id 1)
      ;; element ノード
      (let [tag (vector-get node 1)
            attrs (vector-get node 2)
            children (vector-get node 3)]
        (if (= (void-element? tag) 1)
          ;; void element: 閉じタグなし
          (string-concat "<"
            (string-concat tag
              (string-concat (render-attrs attrs) ">")))
          ;; 通常 element: 開始タグ + children + 閉じタグ
          (string-concat "<"
            (string-concat tag
              (string-concat (render-attrs attrs)
                (string-concat ">"
                  (string-concat
                    (render-children-loop children 0 (vector-length children) "")
                    (string-concat "</"
                      (string-concat tag ">")))))))))
      (if (= tag-id 2)
        ;; text ノード (既にエスケープ済み)
        (vector-get node 1)
        (if (= tag-id 3)
          ;; raw ノード (エスケープなし)
          (vector-get node 1)
          "")))))

;; ルートノードを HTML 文字列にレンダリングするエントリポイント
(defn render-template [node]
  (render-node node))

;; === ヘルパー ===

;; ノード vector の全要素をレンダリングして連結する
(defn each-nodes [items]
  (render-children-loop items 0 (vector-length items) ""))

;; 条件付きノードレンダリング (cond=1 の場合のみ出力)
(defn when-node [cond node]
  (if (= cond 1) (render-node node) ""))

;; HTML5 doctype 宣言
(defn doctype [] "<!doctype html>")

;; 検証用 main
(defn main []
  (let [escaped (html-escape "<>&")]
    (do
      (print (if (string-eq escaped "&lt;&gt;&amp;") 1 0))
      (print (string-length escaped))
      0)))
