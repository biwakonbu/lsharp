;; 階乗
(defn fact [n]
  (if (<= n 1)
    1
    (* n (fact (- n 1)))))

(defn main []
  (do
    (print (fact 10))
    (print (fact 5))
    (print (fact 0))))
