;; Vector.ls - L# 標準ライブラリ: 可変長配列操作
;;
;; ビルトイン vector-new, vector-push, vector-get, vector-set, vector-length の
;; ラッパーと高階関数を提供する。

;; === 高階関数 ===

;; 各要素に関数を適用して新しいベクタを返す
(private
  (defn vector-map-impl [f v i len result]
    (if (>= i len)
      result
      (vector-map-impl f v (+ i 1) len
        (vector-push result (f (vector-get v i)))))))

(defn vector-map
  [f v]
  :doc "各要素へ関数を適用した新しいベクタを返す。"
  :params [ (f "各要素へ適用する関数") (v "変換対象のベクタ")]
  :returns "要素が変換された新しいベクタ"
  :example [ (vector-map (fn [x] (+ x 1)) (vector-push (vector-push (vector-new 2) 1) 2))]
  (vector-map-impl f v 0 (vector-length v) (vector-new (vector-length v))))

;; 条件を満たす要素だけを残す
(private
  (defn vector-filter-impl [f v i len result]
    (if (>= i len)
      result
      (if (f (vector-get v i))
        (vector-filter-impl f v (+ i 1) len
          (vector-push result (vector-get v i)))
        (vector-filter-impl f v (+ i 1) len result)))))

(defn vector-filter
  [f v]
  :doc "条件を満たす要素だけを残した新しいベクタを返す。"
  :params [ (f "残す要素を判定する関数") (v "絞り込み対象のベクタ")]
  :returns "条件を満たす要素だけを含むベクタ"
  :example [ (vector-filter (fn [x] (> x 1)) (vector-push (vector-push (vector-new 2) 1) 2))]
  (vector-filter-impl f v 0 (vector-length v) (vector-new 0)))

;; 左畳み込み
(private
  (defn vector-fold-impl [f acc v i len]
    (if (>= i len)
      acc
      (vector-fold-impl f (f acc (vector-get v i)) v (+ i 1) len))))

(defn vector-fold
  [f init v]
  :doc "ベクタを左から順に畳み込む。"
  :params [ (f "畳み込み関数") (init "初期値") (v "対象のベクタ")]
  :returns "畳み込み後の値"
  :example [ (vector-fold (fn [acc x] (+ acc x)) 0 (vector-push (vector-push (vector-new 2) 1) 2))]
  (vector-fold-impl f init v 0 (vector-length v)))

;; 全要素の合計
(defn vector-sum
  [v]
  :doc "Int ベクタの全要素の合計を返す。"
  :params [ (v "合計したいベクタ")]
  :returns "全要素の合計"
  :example [ (vector-sum (vector-push (vector-push (vector-new 2) 1) 2))]
  (vector-fold (fn [acc x] (+ acc x)) 0 v))

;; === ユーティリティ ===

;; ベクタが空かどうか
(defn vector-empty?
  [v]
  :doc "ベクタが空かどうかを判定する。"
  :params [ (v "判定対象のベクタ")]
  :returns "要素数が 0 なら 1、そうでなければ 0"
  :example [ (vector-empty? (vector-new 0))]
  (== (vector-length v) 0))

;; エントリポイント (テスト用)
(private
  (defn main []
    (let [v (vector-push (vector-push (vector-push (vector-new 4) 1) 2) 3)]
      (do
        (print (vector-length v))
        (print (vector-get v 0))
        (print (vector-get v 2))
        0))))
