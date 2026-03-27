;; Set.ls - L# 標準ライブラリ: 集合操作
;;
;; HashMap ベースの HashSet を提供する。
;; 値として 1 を格納し、キーの有無で集合を表現する。

;; === 基本操作 ===

;; 空の集合を作る
(defn set-new
  []
  :doc "空の集合を作る。"
  :returns "要素を持たない新しい集合"
  :example [(set-new)]
  (map-new))

;; 要素を追加
(defn set-add
  [s x]
  :doc "集合へ要素を追加する。"
  :params [(s "更新対象の集合") (x "追加する要素")]
  :returns "x を含む新しい集合"
  :example [(set-add (set-new) 10)]
  (map-insert s x 1))

;; 要素を含むか
(defn set-contains?
  [s x]
  :doc "集合が要素を含むかどうかを判定する。"
  :params [(s "判定対象の集合") (x "検索する要素")]
  :returns "x を含むなら 1、そうでなければ 0"
  :example [(set-contains? (set-add (set-new) 10) 10)]
  (map-contains? s x))

;; 要素を削除
(defn set-remove
  [s x]
  :doc "集合から要素を削除する。"
  :params [(s "更新対象の集合") (x "削除する要素")]
  :returns "x を除いた新しい集合"
  :example [(set-remove (set-add (set-new) 10) 10)]
  (map-remove s x))

;; 集合のサイズ
(defn set-size
  [s]
  :doc "集合に含まれる要素数を返す。"
  :params [(s "対象の集合")]
  :returns "集合の要素数"
  :example [(set-size (set-add (set-new) 10))]
  (map-size s))

;; 集合が空かどうか
(defn set-empty?
  [s]
  :doc "集合が空かどうかを判定する。"
  :params [(s "判定対象の集合")]
  :returns "空なら 1、そうでなければ 0"
  :example [(set-empty? (set-new))]
  (== (set-size s) 0))

;; エントリポイント (テスト用)
(private
  (defn main []
    (let [s (set-new)
          s1 (set-add s 10)
          s2 (set-add s1 20)
          s3 (set-add s2 30)]
      (do
        (print (set-size s3))
        (print (set-contains? s3 20))
        (print (set-contains? s3 99))
        0))))
