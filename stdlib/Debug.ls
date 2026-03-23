;; Debug.ls - L# 標準ライブラリ: デバッグユーティリティ
;;
;; デバッグ用の出力・アサーション関数を提供する。

;; === デバッグ出力 ===

;; 値をそのまま出力して返す (デバッグ用)
(defn debug-print [x]
  (do
    (print x)
    x))

;; === アサーション ===

;; 条件が真でなければ 0 を返す (型一致のため両分岐 Int)
;; 将来的に panic ビルトインが追加されたら置き換え予定
(defn assert [cond]
  (if cond 0 0))

;; 二値が等しいか検証
(defn assert-eq [a b]
  (assert (== a b)))

;; 二値が等しくないか検証
(defn assert-ne [a b]
  (assert (!= a b)))

;; 値が正であるか検証
(defn assert-positive [x]
  (assert (> x 0)))

;; エントリポイント (テスト用)
(defn main []
  (do
    (assert true)
    (assert-eq 42 42)
    (assert-ne 1 2)
    (assert-positive 10)
    (print (debug-print 99))
    0))
