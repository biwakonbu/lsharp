;; List.ls - L# 標準ライブラリ: リスト操作
;;
;; 再帰的なリスト型と基本的な操作関数を提供する。

;; === リスト型定義 ===

(type (List a) (Cons a (List a)) Nil)

;; === 基本操作 ===

;; リストの長さを返す
(defn length
  [xs]
  :doc "リストの要素数を返す。"
  :params [(xs "長さを数えるリスト")]
  :returns "xs に含まれる要素数"
  :example [(length (Cons 1 (Cons 2 Nil)))]
  (match xs
    [Nil 0]
    [(Cons _ t) (+ 1 (length t))]))

;; リストの先頭要素を返す (空リストの場合はデフォルト値)
(defn head
  [xs default]
  :doc "リストの先頭要素を返し、空リストならデフォルト値を返す。"
  :params [(xs "対象のリスト") (default "空リスト時の代替値")]
  :returns "先頭要素、または default"
  :example [(head (Cons 1 Nil) 0)]
  (match xs
    [Nil default]
    [(Cons h _) h]))

;; リストの先頭を除いた残りを返す (空リストの場合は Nil)
(defn tail
  [xs]
  :doc "リストの先頭を除いた残りを返す。"
  :params [(xs "対象のリスト")]
  :returns "先頭を除いたリスト。空なら Nil"
  :example [(tail (Cons 1 (Cons 2 Nil)))]
  (match xs
    [Nil Nil]
    [(Cons _ t) t]))

;; === 変換操作 ===

;; 各要素に関数を適用する
(defn map
  [f xs]
  :doc "各要素へ関数を適用した新しいリストを返す。"
  :params [(f "各要素へ適用する関数") (xs "変換対象のリスト")]
  :returns "各要素が変換されたリスト"
  :example [(map (fn [x] (+ x 1)) (Cons 1 (Cons 2 Nil)))]
  (match xs
    [Nil Nil]
    [(Cons h t) (Cons (f h) (map f t))]))

;; 条件を満たす要素だけを残す
(defn filter
  [f xs]
  :doc "条件を満たす要素だけを残したリストを返す。"
  :params [(f "残す要素を判定する関数") (xs "絞り込み対象のリスト")]
  :returns "条件を満たす要素だけを含むリスト"
  :example [(filter (fn [x] (> x 1)) (Cons 1 (Cons 2 Nil)))]
  (match xs
    [Nil Nil]
    [(Cons h t) (if (f h) (Cons h (filter f t)) (filter f t))]))

;; 左畳み込み
(defn fold
  [f init xs]
  :doc "左から順に要素を畳み込む。"
  :params [(f "畳み込み関数") (init "初期値") (xs "対象のリスト")]
  :returns "畳み込み後の値"
  :example [(fold (fn [acc x] (+ acc x)) 0 (Cons 1 (Cons 2 Nil)))]
  (match xs
    [Nil init]
    [(Cons h t) (fold f (f init h) t)]))

;; === 結合操作 ===

;; 2 つのリストを結合する
(defn append
  [xs ys]
  :doc "2 つのリストを連結する。"
  :params [(xs "前半のリスト") (ys "後半のリスト")]
  :returns "xs の末尾に ys を連結したリスト"
  :example [(append (Cons 1 Nil) (Cons 2 Nil))]
  (match xs
    [Nil ys]
    [(Cons h t) (Cons h (append t ys))]))

;; リストを逆順にする
(defn reverse
  [xs]
  :doc "リストの順序を反転する。"
  :params [(xs "反転対象のリスト")]
  :returns "要素順が逆になったリスト"
  :example [(reverse (Cons 1 (Cons 2 Nil)))]
  (fold (fn [acc x] (Cons x acc)) Nil xs))

;; === 検索・判定 ===

;; リストが空かどうか
(defn is-empty
  [xs]
  :doc "リストが空かどうかを判定する。"
  :params [(xs "判定対象のリスト")]
  :returns "空なら 1、そうでなければ 0"
  :example [(is-empty Nil)]
  (match xs
    [Nil 1]
    [(Cons _ _) 0]))

;; 全要素の合計 (Int リスト用)
(defn sum
  [xs]
  :doc "Int リストの全要素の合計を返す。"
  :params [(xs "合計したい Int リスト")]
  :returns "全要素の合計"
  :example [(sum (Cons 1 (Cons 2 Nil)))]
  (fold (fn [acc x] (+ acc x)) 0 xs))

;; 全要素の積 (Int リスト用)
(defn product
  [xs]
  :doc "Int リストの全要素の積を返す。"
  :params [(xs "積を取りたい Int リスト")]
  :returns "全要素の積"
  :example [(product (Cons 2 (Cons 3 Nil)))]
  (fold (fn [acc x] (* acc x)) 1 xs))

;; === ユーティリティ ===

;; n 番目の要素を取得 (0-indexed, 範囲外はデフォルト値)
(defn nth
  [xs n default]
  :doc "0 始まりで n 番目の要素を返し、範囲外ならデフォルト値を返す。"
  :params [(xs "対象のリスト") (n "取得したい位置") (default "範囲外時の代替値")]
  :returns "n 番目の要素、または default"
  :example [(nth (Cons 1 (Cons 2 Nil)) 1 0)]
  (match xs
    [Nil default]
    [(Cons h t) (if (== n 0) h (nth t (- n 1) default))]))

;; 先頭 n 個を取得
(defn take
  [n xs]
  :doc "リストの先頭から n 個の要素を取り出す。"
  :params [(n "取り出す要素数") (xs "対象のリスト")]
  :returns "先頭から最大 n 個の要素を含むリスト"
  :example [(take 2 (Cons 1 (Cons 2 (Cons 3 Nil))))]
  (if (<= n 0) Nil
    (match xs
      [Nil Nil]
      [(Cons h t) (Cons h (take (- n 1) t))])))

;; 先頭 n 個を除去
(defn drop
  [n xs]
  :doc "リストの先頭から n 個の要素を捨てる。"
  :params [(n "捨てる要素数") (xs "対象のリスト")]
  :returns "先頭から n 個を除いた残りのリスト"
  :example [(drop 1 (Cons 1 (Cons 2 Nil)))]
  (if (<= n 0) xs
    (match xs
      [Nil Nil]
      [(Cons _ t) (drop (- n 1) t)])))

;; エントリポイント (ライブラリテスト用)
(private
  (defn main []
    (let [xs (Cons 1 (Cons 2 (Cons 3 Nil)))]
      (do
        ;; length テスト
        (print (length xs))
        ;; head テスト
        (print (head xs 0))
        ;; sum テスト
        (print (sum xs))
        ;; nth テスト
        (print (nth xs 1 0))
        0))))
