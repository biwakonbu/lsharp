;; V4-M1-03-R3 positive boundary: 4097 unrooted allocations cross the
;; initial free-list capacity while the program still returns deterministically.
(defn alloc-unrooted [n total]
  (if (<= n 0)
    total
    (let [value (__alloc 8)]
      (alloc-unrooted (- n 1) total))))

(defn main []
  (print (alloc-unrooted 4097 4097)))
