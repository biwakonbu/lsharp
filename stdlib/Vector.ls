;; Vector.ls - L# 標準ライブラリ: 可変長配列操作
;;
;; ビルトイン vector-new, vector-push, vector-get, vector-set, vector-length の
;; ラッパーと高階関数を提供する。

;; === 高階関数 ===

;; 各要素に関数を適用して新しいベクタを返す
(defn vector-map-impl [f v i len result]
  (if (>= i len)
    result
    (vector-map-impl f v (+ i 1) len
      (vector-push result (f (vector-get v i))))))

(defn vector-map [f v]
  (vector-map-impl f v 0 (vector-length v) (vector-new (vector-length v))))

;; 条件を満たす要素だけを残す
(defn vector-filter-impl [f v i len result]
  (if (>= i len)
    result
    (if (f (vector-get v i))
      (vector-filter-impl f v (+ i 1) len
        (vector-push result (vector-get v i)))
      (vector-filter-impl f v (+ i 1) len result))))

(defn vector-filter [f v]
  (vector-filter-impl f v 0 (vector-length v) (vector-new 0)))

;; 左畳み込み
(defn vector-fold-impl [f acc v i len]
  (if (>= i len)
    acc
    (vector-fold-impl f (f acc (vector-get v i)) v (+ i 1) len)))

(defn vector-fold [f init v]
  (vector-fold-impl f init v 0 (vector-length v)))

;; 全要素の合計
(defn vector-sum [v]
  (vector-fold (fn [acc x] (+ acc x)) 0 v))

;; === ユーティリティ ===

;; ベクタが空かどうか
(defn vector-empty? [v]
  (== (vector-length v) 0))

;; エントリポイント (テスト用)
(defn main []
  (let [v (vector-push (vector-push (vector-push (vector-new 4) 1) 2) 3)]
    (do
      (print (vector-length v))
      (print (vector-get v 0))
      (print (vector-get v 2))
      0)))
