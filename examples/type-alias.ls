; 型エイリアスサンプル
(type-alias Str String)
(type-alias Natural Int)

(defn add-natural [(: x Natural) (: y Natural)] : Natural
  (+ x y))

(defn main []
  (print (add-natural 3 4)))
