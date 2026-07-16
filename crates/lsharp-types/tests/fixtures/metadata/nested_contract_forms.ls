(module App
  (module Sub
    (defn succ [x]
      :example [(succ 0)]
      :invariant (= result (+ x 1))
      (+ x 1))))
