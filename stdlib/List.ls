;; List.ls - L# 標準ライブラリ: リスト操作
;;
;; 再帰的なリスト型と基本的な操作関数を提供する。

;; === リスト型定義 ===

(type (List a) (Cons a (List a)) Nil)

;; === 基本操作 ===

;; リストの長さを返す
(defn length [xs]
  (match xs
    [Nil 0]
    [(Cons _ t) (+ 1 (length t))]))

;; リストの先頭要素を返す (空リストの場合はデフォルト値)
(defn head [xs default]
  (match xs
    [Nil default]
    [(Cons h _) h]))

;; リストの先頭を除いた残りを返す (空リストの場合は Nil)
(defn tail [xs]
  (match xs
    [Nil Nil]
    [(Cons _ t) t]))

;; === 変換操作 ===

;; 各要素に関数を適用する
(defn map [f xs]
  (match xs
    [Nil Nil]
    [(Cons h t) (Cons (f h) (map f t))]))

;; 条件を満たす要素だけを残す
(defn filter [f xs]
  (match xs
    [Nil Nil]
    [(Cons h t) (if (f h) (Cons h (filter f t)) (filter f t))]))

;; 左畳み込み
(defn fold [f init xs]
  (match xs
    [Nil init]
    [(Cons h t) (fold f (f init h) t)]))

;; === 結合操作 ===

;; 2 つのリストを結合する
(defn append [xs ys]
  (match xs
    [Nil ys]
    [(Cons h t) (Cons h (append t ys))]))

;; リストを逆順にする
(defn reverse [xs]
  (fold (fn [acc x] (Cons x acc)) Nil xs))

;; === 検索・判定 ===

;; リストが空かどうか
(defn is-empty [xs]
  (match xs
    [Nil 1]
    [(Cons _ _) 0]))

;; 全要素の合計 (Int リスト用)
(defn sum [xs]
  (fold (fn [acc x] (+ acc x)) 0 xs))

;; 全要素の積 (Int リスト用)
(defn product [xs]
  (fold (fn [acc x] (* acc x)) 1 xs))

;; === ユーティリティ ===

;; n 番目の要素を取得 (0-indexed, 範囲外はデフォルト値)
(defn nth [xs n default]
  (match xs
    [Nil default]
    [(Cons h t) (if (== n 0) h (nth t (- n 1) default))]))

;; 先頭 n 個を取得
(defn take [n xs]
  (if (<= n 0) Nil
    (match xs
      [Nil Nil]
      [(Cons h t) (Cons h (take (- n 1) t))])))

;; 先頭 n 個を除去
(defn drop [n xs]
  (if (<= n 0) xs
    (match xs
      [Nil Nil]
      [(Cons _ t) (drop (- n 1) t)])))

;; エントリポイント (ライブラリテスト用)
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
      0)))
