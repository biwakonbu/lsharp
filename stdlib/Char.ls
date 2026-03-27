;; Char.ls - L# 標準ライブラリ: 文字判定
;;
;; ASCII 文字コードに基づく文字判定関数を提供する。
;; 文字は整数 (ASCII コードポイント) として扱う。

;; === 文字判定 ===

;; 数字か (0-9: ASCII 48-57)
(defn is-digit
  [c]
  :doc "ASCII コードが数字かどうかを判定する。"
  :params [(c "判定対象の ASCII コード")]
  :returns "0-9 の範囲なら true、そうでなければ false"
  :example [(is-digit 48)]
  (if (>= c 48)
    (<= c 57)
    false))

;; 大文字アルファベットか (A-Z: ASCII 65-90)
(defn is-upper
  [c]
  :doc "ASCII コードが大文字アルファベットかどうかを判定する。"
  :params [(c "判定対象の ASCII コード")]
  :returns "A-Z の範囲なら true、そうでなければ false"
  :example [(is-upper 65)]
  (if (>= c 65)
    (<= c 90)
    false))

;; 小文字アルファベットか (a-z: ASCII 97-122)
(defn is-lower
  [c]
  :doc "ASCII コードが小文字アルファベットかどうかを判定する。"
  :params [(c "判定対象の ASCII コード")]
  :returns "a-z の範囲なら true、そうでなければ false"
  :example [(is-lower 97)]
  (if (>= c 97)
    (<= c 122)
    false))

;; アルファベットか (A-Z or a-z)
(defn is-alpha
  [c]
  :doc "ASCII コードが英字かどうかを判定する。"
  :params [(c "判定対象の ASCII コード")]
  :returns "英字なら true、そうでなければ false"
  :example [(is-alpha 65)]
  (if (is-upper c)
    true
    (is-lower c)))

;; 英数字か
(defn is-alphanumeric
  [c]
  :doc "ASCII コードが英数字かどうかを判定する。"
  :params [(c "判定対象の ASCII コード")]
  :returns "英字または数字なら true、そうでなければ false"
  :example [(is-alphanumeric 48)]
  (if (is-alpha c)
    true
    (is-digit c)))

;; 空白文字か (space=32, tab=9, newline=10, return=13)
(defn is-whitespace
  [c]
  :doc "ASCII コードが代表的な空白文字かどうかを判定する。"
  :params [(c "判定対象の ASCII コード")]
  :returns "space, tab, newline, return のいずれかなら true、そうでなければ false"
  :example [(is-whitespace 32)]
  (if (== c 32) true
    (if (== c 9) true
      (if (== c 10) true
        (== c 13)))))

;; エントリポイント (テスト用)
(private
  (defn main []
    (do
      (print (is-digit 48))
      (print (is-digit 65))
      (print (is-alpha 65))
      (print (is-alpha 48))
      (print (is-whitespace 32))
      0)))
