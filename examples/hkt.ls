; hkt.ls - 高カインド型（Higher-Kinded Types）のサンプル
; 注: GC struct 型は wasmtime で未サポートのため、型チェックのみ検証。main は print のスタブ。

(type (Maybe a)
  (Just a)
  Nothing)

(trait (Functor f)
  (defn fmap [func fa] : (f b)))

(defn identity [x] x)

(defn main []
  (print (identity 42)))
