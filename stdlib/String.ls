;; String.ls - L# 標準ライブラリ: 文字列操作
;;
;; ビルトインの文字列操作関数を基に、より高レベルな操作を提供する。
;; ビルトイン: string-length, string-concat, string-eq,
;;            string-char-at, substring, int-to-string

;; === 判定関数 ===

;; 文字列が空かどうか
(defn string-empty?
  [s]
  :doc "文字列が空かどうかを判定する。"
  :params [(s "判定対象の文字列")]
  :returns "空文字列なら 1、そうでなければ 0"
  :example [(string-empty? "")]
  (== (string-length s) 0))

;; 文字列が指定のプレフィックスで始まるか
(defn starts-with
  [s prefix]
  :doc "文字列が指定したプレフィックスで始まるかどうかを判定する。"
  :params [(s "判定対象の文字列") (prefix "期待する接頭辞")]
  :returns "prefix で始まるなら true、そうでなければ false"
  :example [(starts-with "hello world" "hello")]
  (if (> (string-length prefix) (string-length s))
    false
    (string-eq (substring s 0 (string-length prefix)) prefix)))

;; 文字列が指定のサフィックスで終わるか
(defn ends-with
  [s suffix]
  :doc "文字列が指定したサフィックスで終わるかどうかを判定する。"
  :params [(s "判定対象の文字列") (suffix "期待する接尾辞")]
  :returns "suffix で終わるなら true、そうでなければ false"
  :example [(ends-with "hello world" "world")]
  (let [slen (string-length s)
        suflen (string-length suffix)]
    (if (> suflen slen)
      false
      (string-eq (substring s (- slen suflen) slen) suffix))))

;; === 変換関数 ===

;; 文字列を繰り返す
(defn string-repeat
  [s n]
  :doc "文字列を指定回数だけ連結した文字列を返す。"
  :params [(s "繰り返す文字列") (n "繰り返し回数")]
  :returns "s を n 回連結した文字列"
  :example [(string-repeat "ha" 3)]
  (if (<= n 0) ""
    (if (== n 1) s
      (string-concat s (string-repeat s (- n 1))))))

;; 2 つの文字列を区切り文字で結合
(defn string-join2
  [a b sep]
  :doc "2 つの文字列を区切り文字で連結する。"
  :params [(a "前半の文字列") (b "後半の文字列") (sep "区切り文字列")]
  :returns "a, sep, b を連結した文字列"
  :example [(string-join2 "foo" "bar" "/")]
  (string-concat (string-concat a sep) b))

;; === 検索関数 ===

;; string-index-of の内部ヘルパー: 位置 i から検索
(private
  (defn string-search-from [haystack needle hlen nlen i]
    (if (> (+ i nlen) hlen)
      (- 0 1)
      (if (string-eq (substring haystack i (+ i nlen)) needle)
        i
        (string-search-from haystack needle hlen nlen (+ i 1))))))

;; 文字列内に部分文字列が含まれるか (O(n*m))
;; 見つかった位置を返す。見つからない場合は -1
(defn string-index-of
  [haystack needle]
  :doc "部分文字列が最初に出現する位置を返す。"
  :params [(haystack "検索対象の文字列") (needle "探したい部分文字列")]
  :returns "最初の出現位置。見つからなければ -1"
  :example [(string-index-of "hello world" "lo")]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (> nlen hlen)
      (- 0 1)
      (string-search-from haystack needle hlen nlen 0))))

;; 文字列内に部分文字列が含まれるか (bool)
(defn string-contains
  [haystack needle]
  :doc "部分文字列を含むかどうかを判定する。"
  :params [(haystack "検索対象の文字列") (needle "探したい部分文字列")]
  :returns "needle を含むなら 1、そうでなければ 0"
  :example [(string-contains "hello world" "world")]
  (if (>= (string-index-of haystack needle) 0) 1 0))

;; === 数値変換 ===

;; Bool 値を文字列に変換
(defn bool-to-string
  [b]
  :doc "真偽値を \"true\" または \"false\" の文字列へ変換する。"
  :params [(b "変換対象の真偽値")]
  :returns "b に対応する文字列表現"
  :example [(bool-to-string true)]
  (if b "true" "false"))

;; エントリポイント (ライブラリテスト用)
(private
  (defn main []
    (do
      ;; string-empty? テスト
      (print (string-empty? ""))
      (print (string-empty? "hello"))
      ;; starts-with テスト
      (print (starts-with "hello world" "hello"))
      (print (starts-with "hello" "hello world"))
      ;; ends-with テスト
      (print (ends-with "hello world" "world"))
      ;; string-contains テスト
      (print (string-contains "hello world" "lo wo"))
      (print (string-contains "hello" "xyz"))
      0)))
