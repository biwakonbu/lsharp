;; Core.ls - L# 標準ライブラリ: 基本ユーティリティ
;;
;; Bool 操作、数学関数、Option/Result 型を提供する。

;; === Bool ユーティリティ ===
;; 注: and, or, not はビルトイン演算子として提供済み

;; 排他的論理和
(defn xor
  [a b]
  :doc "2 つの真偽値に対する排他的論理和を返す。"
  :params [ (a "左オペランド") (b "右オペランド")]
  :returns "片方だけが真なら 1、そうでなければ 0"
  :example [ (xor true false)]
  (if a (if b 0 1) (if b 1 0)))

;; === 基本数学関数 ===

;; 絶対値
(defn abs
  [x]
  :doc "整数の絶対値を返す。"
  :params [ (x "対象の整数")]
  :returns "x の絶対値"
  :example [ (abs (- 0 5))]
  (if (< x 0) (- 0 x) x))

;; 最大値
(defn max
  [a b]
  :doc "2 つの値のうち大きい方を返す。"
  :params [ (a "比較対象 1") (b "比較対象 2")]
  :returns "a と b の最大値"
  :example [ (max 3 7)]
  (if (> a b) a b))

;; 最小値
(defn min
  [a b]
  :doc "2 つの値のうち小さい方を返す。"
  :params [ (a "比較対象 1") (b "比較対象 2")]
  :returns "a と b の最小値"
  :example [ (min 3 7)]
  (if (< a b) a b))

;; クランプ: lo <= x <= hi の範囲に収める
(defn clamp
  [x lo hi]
  :doc "値を指定した下限と上限の範囲に収める。"
  :params [ (x "対象値") (lo "下限値") (hi "上限値")]
  :returns "lo 以上 hi 以下に丸められた値"
  :example [ (clamp 15 0 10)]
  (max lo (min x hi)))

;; === Option 型 ===
;; 値の有無を表す型

(type (Option a) (Some a) None)

;; Option から値を取り出す。None の場合はデフォルト値を返す
(defn unwrap
  [opt default]
  :doc "Option から値を取り出し、None の場合はデフォルト値を返す。"
  :params [ (opt "取り出し対象の Option") (default "None の場合に返す値")]
  :returns "Some の中身、または default"
  :example [ (unwrap (Some 42) 0)]
  (match opt
    [ (Some x) x]
    [None default]))

;; Option に関数を適用する
(defn map-option
  [f opt]
  :doc "Option が Some の場合にだけ関数を適用する。"
  :params [ (f "Some の値へ適用する関数") (opt "変換対象の Option")]
  :returns "変換後の Option"
  :example [ (map-option (fn [x] (+ x 1)) (Some 1))]
  (match opt
    [ (Some x) (Some (f x))]
    [None None]))

;; Option が Some かどうか (1 = true, 0 = false)
(defn is-some
  [opt]
  :doc "Option が Some かどうかを判定する。"
  :params [ (opt "判定対象の Option")]
  :returns "Some なら 1、None なら 0"
  :example [ (is-some (Some 1))]
  (match opt
    [ (Some _) 1]
    [None 0]))

;; Option が None かどうか (1 = true, 0 = false)
(defn is-none
  [opt]
  :doc "Option が None かどうかを判定する。"
  :params [ (opt "判定対象の Option")]
  :returns "None なら 1、Some なら 0"
  :example [ (is-none None)]
  (match opt
    [ (Some _) 0]
    [None 1]))

;; === Result 型 ===
;; 成功/失敗を表す型

(type (Result a e) (Ok a) (Err e))

;; Result から成功値を取り出す。Err の場合はデフォルト値を返す
(defn unwrap-ok
  [res default]
  :doc "Result から成功値を取り出し、Err の場合はデフォルト値を返す。"
  :params [ (res "取り出し対象の Result") (default "Err の場合に返す値")]
  :returns "Ok の中身、または default"
  :example [ (unwrap-ok (Ok 42) 0)]
  (match res
    [ (Ok x) x]
    [ (Err _) default]))

;; Result に関数を適用する (Ok の場合のみ)
(defn map-result
  [f res]
  :doc "Result が Ok の場合にだけ関数を適用する。"
  :params [ (f "Ok の値へ適用する関数") (res "変換対象の Result")]
  :returns "変換後の Result"
  :example [ (map-result (fn [x] (+ x 1)) (Ok 1))]
  (match res
    [ (Ok x) (Ok (f x))]
    [ (Err e) (Err e)]))

;; Result が Ok かどうか
(defn is-ok
  [res]
  :doc "Result が Ok かどうかを判定する。"
  :params [ (res "判定対象の Result")]
  :returns "Ok なら 1、Err なら 0"
  :example [ (is-ok (Ok 1))]
  (match res
    [ (Ok _) 1]
    [ (Err _) 0]))

;; Result が Err かどうか
(defn is-err
  [res]
  :doc "Result が Err かどうかを判定する。"
  :params [ (res "判定対象の Result")]
  :returns "Err なら 1、Ok なら 0"
  :example [ (is-err (Err 1))]
  (match res
    [ (Ok _) 0]
    [ (Err _) 1]))

;; === 関数合成ユーティリティ ===

;; 恒等関数
(defn identity
  [x]
  :doc "受け取った値をそのまま返す。"
  :params [ (x "返したい値")]
  :returns "入力と同じ値"
  :example [ (identity 42)]
  x)

;; 定数関数: 常に x を返す関数を返す
(defn const
  [x]
  :doc "引数を無視して常に同じ値を返す関数を作る。"
  :params [ (x "固定で返す値")]
  :returns "任意の引数に対して x を返す関数"
  :example [ ( (const 1) 99)]
  (fn [_] x))

;; 関数を 2 回適用する
(defn twice
  [f x]
  :doc "関数を同じ値に 2 回連続で適用する。"
  :params [ (f "2 回適用する関数") (x "初回入力")]
  :returns "f を 2 回適用した結果"
  :example [ (twice (fn [n] (+ n 1)) 3)]
  (f (f x)))

;; エントリポイント (ライブラリテスト用)
(private
  (defn main []
    (do
      ;; abs テスト
      (print (abs (- 0 5)))
      ;; max / min テスト
      (print (max 3 7))
      (print (min 3 7))
      ;; clamp テスト
      (print (clamp 15 0 10))
      (print (clamp (- 0 5) 0 10))
      0)))
