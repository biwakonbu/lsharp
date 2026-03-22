; trait-where.ls - Where 句付きトレイト制約のサンプル

(trait (Addable a)
  (defn add-val [x y] : Int))

(defn sum-two [x y]
  :where [(Addable a)]
  (+ x y))

(defn main []
  (print (sum-two 10 20)))
