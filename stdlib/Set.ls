;; Set.ls - L# 標準ライブラリ: 集合操作
;;
;; HashMap ベースの HashSet を提供する。
;; 値として 1 を格納し、キーの有無で集合を表現する。

;; === 基本操作 ===

;; 空の集合を作る
(defn set-new []
  (map-new))

;; 要素を追加
(defn set-add [s x]
  (map-insert s x 1))

;; 要素を含むか
(defn set-contains? [s x]
  (map-contains? s x))

;; 要素を削除
(defn set-remove [s x]
  (map-remove s x))

;; 集合のサイズ
(defn set-size [s]
  (map-size s))

;; 集合が空かどうか
(defn set-empty? [s]
  (== (set-size s) 0))

;; エントリポイント (テスト用)
(defn main []
  (let [s (set-new)
        s1 (set-add s 10)
        s2 (set-add s1 20)
        s3 (set-add s2 30)]
    (do
      (print (set-size s3))
      (print (set-contains? s3 20))
      (print (set-contains? s3 99))
      0)))
