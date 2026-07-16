(defn succ
  [x]
  :example [(= (succ 0) 1) (= (succ 1) 999)]
  :invariant (= result (+ x 1))
  (+ x 1))
