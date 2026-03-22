; モジュールサンプル
(module Math)

(defn add [x y] (+ x y))
(defn mul [x y] (* x y))

(defn main []
  (print (add (mul 3 4) 5)))
