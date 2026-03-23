;; Map.ls - L# 標準ライブラリ: ハッシュマップ操作
;;
;; ビルトイン map-new, map-insert, map-get, map-contains?, map-remove, map-size の
;; ラッパーを提供する。

;; === ユーティリティ ===

;; マップが空かどうか
(defn map-empty? [m]
  (== (map-size m) 0))

;; キーに関数を適用してデフォルト値を返す (キーが存在しない場合)
(defn map-get-or [m key default]
  (if (map-contains? m key)
    (map-get m key)
    default))

;; 複数のキー・値ペアを挿入
(defn map-insert-all-impl [m keys vals i len]
  (if (>= i len)
    m
    (map-insert-all-impl
      (map-insert m (vector-get keys i) (vector-get vals i))
      keys vals (+ i 1) len)))

;; エントリポイント (テスト用)
(defn main []
  (let [m (map-new)
        m1 (map-insert m 1 100)
        m2 (map-insert m1 2 200)]
    (do
      (print (map-size m2))
      (print (map-get m2 1))
      (print (map-get m2 2))
      0)))
