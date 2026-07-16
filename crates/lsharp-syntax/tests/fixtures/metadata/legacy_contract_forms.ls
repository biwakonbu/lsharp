(defn succ [x]
  :example [(succ 0)]
  :invariant (= result (+ x 1))
  :example [(succ 1)]
  (+ x 1))
