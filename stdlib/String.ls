;; String.ls - L# 標準ライブラリ: 文字列操作
;;
;; ビルトインの文字列操作関数を基に、より高レベルな操作を提供する。
;; ビルトイン: string-length, string-concat, string-eq,
;;            string-char-at, substring, int-to-string

;; === 判定関数 ===

;; 文字列が空かどうか
(defn string-empty? [s]
  (== (string-length s) 0))

;; 文字列が指定のプレフィックスで始まるか
(defn starts-with [s prefix]
  (if (> (string-length prefix) (string-length s))
    false
    (string-eq (substring s 0 (string-length prefix)) prefix)))

;; 文字列が指定のサフィックスで終わるか
(defn ends-with [s suffix]
  (let [slen (string-length s)
        suflen (string-length suffix)]
    (if (> suflen slen)
      false
      (string-eq (substring s (- slen suflen) slen) suffix))))

;; === 変換関数 ===

;; 文字列を繰り返す
(defn string-repeat [s n]
  (if (<= n 0) ""
    (if (== n 1) s
      (string-concat s (string-repeat s (- n 1))))))

;; 2 つの文字列を区切り文字で結合
(defn string-join2 [a b sep]
  (string-concat (string-concat a sep) b))

;; === 検索関数 ===

;; string-index-of の内部ヘルパー: 位置 i から検索
(defn string-search-from [haystack needle hlen nlen i]
  (if (> (+ i nlen) hlen)
    (- 0 1)
    (if (string-eq (substring haystack i (+ i nlen)) needle)
      i
      (string-search-from haystack needle hlen nlen (+ i 1)))))

;; 文字列内に部分文字列が含まれるか (O(n*m))
;; 見つかった位置を返す。見つからない場合は -1
(defn string-index-of [haystack needle]
  (let [hlen (string-length haystack)
        nlen (string-length needle)]
    (if (> nlen hlen)
      (- 0 1)
      (string-search-from haystack needle hlen nlen 0))))

;; 文字列内に部分文字列が含まれるか (bool)
(defn string-contains [haystack needle]
  (if (>= (string-index-of haystack needle) 0) 1 0))

;; === 数値変換 ===

;; Bool 値を文字列に変換
(defn bool-to-string [b]
  (if b "true" "false"))

;; エントリポイント (ライブラリテスト用)
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
    0))
