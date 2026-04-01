(defn test-multi-arg [n] (let [r (ref-new n)] (do (if (> n 0) (do (ref-set r n) (if (> n 1) (do (ref-set r n) (if (> n 2) (do (ref-set r n) (if (> n 3) (do (ref-set r n) 0) 0)) 0)) 0)) 0) (ref-get r))))
(defn main [] (test-multi-arg 5))
