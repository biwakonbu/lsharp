; gadt.ls - 代数的データ型のパターンマッチサンプル
; 注: GC struct 型は wasmtime で未サポートのため、型チェックのみ検証。main は print のスタブ。

(type (Expr a)
  (LitInt Int)
  (Add (Expr Int) (Expr Int)))

(defn eval-int [e]
  (match e
    [(LitInt n) n]
    [(Add l r) (+ (eval-int l) (eval-int r))]))

(defn main []
  (print 42))
