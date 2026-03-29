;; Debug.ls - L# 標準ライブラリ: デバッグユーティリティ
;;
;; デバッグ用の出力・アサーション関数を提供する。

;; === デバッグ出力 ===

;; 値をそのまま出力して返す (デバッグ用)
(defn debug-print
  [x]
  :doc "値を出力しつつ、その値をそのまま返す。"
  :params [ (x "出力したい値")]
  :returns "入力と同じ値"
  :example [ (debug-print 99)]
  (do
    (print x)
    x))

;; === アサーション ===

;; 条件が真でなければ 0 を返す (型一致のため両分岐 Int)
;; 将来的に panic ビルトインが追加されたら置き換え予定
(defn assert
  [cond]
  :doc "条件が真であることを表明する。"
  :params [ (cond "検証したい条件")]
  :returns "現状は常に 0"
  :example [ (assert true)]
  (if cond 0 0))

;; 二値が等しいか検証
(defn assert-eq
  [a b]
  :doc "2 つの値が等しいことを表明する。"
  :params [ (a "比較対象 1") (b "比較対象 2")]
  :returns "現状は常に 0"
  :example [ (assert-eq 42 42)]
  (assert (== a b)))

;; 二値が等しくないか検証
(defn assert-ne
  [a b]
  :doc "2 つの値が等しくないことを表明する。"
  :params [ (a "比較対象 1") (b "比較対象 2")]
  :returns "現状は常に 0"
  :example [ (assert-ne 1 2)]
  (assert (!= a b)))

;; 値が正であるか検証
(defn assert-positive
  [x]
  :doc "値が正であることを表明する。"
  :params [ (x "検証したい整数")]
  :returns "現状は常に 0"
  :example [ (assert-positive 10)]
  (assert (> x 0)))

;; エントリポイント (テスト用)
(private
  (defn main []
    (do
      (assert true)
      (assert-eq 42 42)
      (assert-ne 1 2)
      (assert-positive 10)
      (print (debug-print 99))
      0)))
