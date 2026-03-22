; constrained.ls - L# 制約付き型のサンプル

; 自然数（0以上の整数）
(type-constrained Natural Int
  :constraints [(>= 0)])

; パーセンテージ（0-100の整数）
(type-constrained Percentage Int
  :constraints [(>= 0) (<= 100)])

; ポート番号（1-65535の整数）
(type-constrained Port Int
  :constraints [(range 1 65535)])

; 優先度（1, 2, 3のいずれか）
(type-constrained Priority Int
  :constraints [(one-of [1 2 3])])

; 関数定義
(defn safe-add [x y]
  (+ x y))

(defn main []
  (safe-add 10 20))
