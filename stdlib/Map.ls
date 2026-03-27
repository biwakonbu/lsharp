;; Map.ls - L# 標準ライブラリ: ハッシュマップ操作
;;
;; ビルトイン map-new, map-insert, map-get, map-contains?, map-remove, map-size の
;; ラッパーを提供する。

;; === ユーティリティ ===

;; マップが空かどうか
(defn map-empty?
  [m]
  :doc "マップが空かどうかを判定する。"
  :params [(m "判定対象のマップ")]
  :returns "要素数が 0 なら 1、そうでなければ 0"
  :example [(map-empty? (map-new))]
  (== (map-size m) 0))

;; キーに関数を適用してデフォルト値を返す (キーが存在しない場合)
;; map-contains? は Int (0/1) を返すため == で比較する
(defn map-get-or
  [m key default]
  :doc "キーが存在すれば対応する値を返し、存在しなければデフォルト値を返す。"
  :params [(m "検索対象のマップ") (key "取得したいキー") (default "キー不在時の代替値")]
  :returns "キーに対応する値、または default"
  :example [(map-get-or (map-insert (map-new) 1 100) 1 0)]
  (let [has (map-contains? m key)]
    (if (== has 1)
      (map-get m key)
      default)))

;; 複数のキー・値ペアを挿入
(private
  (defn map-insert-all-impl [m keys vals i len]
    (if (>= i len)
      m
      (map-insert-all-impl
        (map-insert m (vector-get keys i) (vector-get vals i))
        keys vals (+ i 1) len))))

;; エントリポイント (テスト用)
(private
  (defn main []
    (let [m (map-new)
          m1 (map-insert m 1 100)
          m2 (map-insert m1 2 200)]
      (do
        (print (map-size m2))
        (print (map-get m2 1))
        (print (map-get m2 2))
        0))))
