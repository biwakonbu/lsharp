;; Core.ls - L# 標準ライブラリ: 基本ユーティリティ
;;
;; Bool 操作、数学関数、Option/Result 型を提供する。

;; === Bool ユーティリティ ===
;; 注: and, or, not はビルトイン演算子として提供済み

;; 排他的論理和
(defn xor [a b] (if a (if b 0 1) (if b 1 0)))

;; === 基本数学関数 ===

;; 絶対値
(defn abs [x] (if (< x 0) (- 0 x) x))

;; 最大値
(defn max [a b] (if (> a b) a b))

;; 最小値
(defn min [a b] (if (< a b) a b))

;; クランプ: lo <= x <= hi の範囲に収める
(defn clamp [x lo hi] (max lo (min x hi)))

;; === Option 型 ===
;; 値の有無を表す型

(type (Option a) (Some a) None)

;; Option から値を取り出す。None の場合はデフォルト値を返す
(defn unwrap [opt default]
  (match opt
    [(Some x) x]
    [None default]))

;; Option に関数を適用する
(defn map-option [f opt]
  (match opt
    [(Some x) (Some (f x))]
    [None None]))

;; Option が Some かどうか (1 = true, 0 = false)
(defn is-some [opt]
  (match opt
    [(Some _) 1]
    [None 0]))

;; Option が None かどうか (1 = true, 0 = false)
(defn is-none [opt]
  (match opt
    [(Some _) 0]
    [None 1]))

;; === Result 型 ===
;; 成功/失敗を表す型

(type (Result a e) (Ok a) (Err e))

;; Result から成功値を取り出す。Err の場合はデフォルト値を返す
(defn unwrap-ok [res default]
  (match res
    [(Ok x) x]
    [(Err _) default]))

;; Result に関数を適用する (Ok の場合のみ)
(defn map-result [f res]
  (match res
    [(Ok x) (Ok (f x))]
    [(Err e) (Err e)]))

;; Result が Ok かどうか
(defn is-ok [res]
  (match res
    [(Ok _) 1]
    [(Err _) 0]))

;; Result が Err かどうか
(defn is-err [res]
  (match res
    [(Ok _) 0]
    [(Err _) 1]))

;; === 関数合成ユーティリティ ===

;; 恒等関数
(defn identity [x] x)

;; 定数関数: 常に x を返す関数を返す
(defn const [x] (fn [_] x))

;; 関数を 2 回適用する
(defn twice [f x] (f (f x)))

;; エントリポイント (ライブラリテスト用)
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
    0))
