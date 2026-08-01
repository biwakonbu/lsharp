;; V4-M1-03-R1 positive boundary: map insert, size, and membership keep
;; deterministic observable semantics across the full Wasm runtime path.
(defn main []
  (let [m0 (map-new)
    m1 (map-insert m0 10 1)
    m2 (map-insert m1 20 1)
    m3 (map-insert m2 30 1)]
    (do
      (print (map-size m3))
      (print (map-contains? m3 20))
      (print (map-contains? m3 99))
      0)))
