;; V4-M1-03-R2 positive boundary: a captured heap value survives an
;; allocating helper before the closure is called.
(defn churn [n]
  (if (<= n 0)
    0
    (let [discarded (string-concat "left" "right")]
      (do
        (string-length discarded)
        (churn (- n 1))))))

(defn make-keeper []
  (let [s (string-concat "keep" "!")]
    (fn [_] (string-length s))))

(defn apply [f x] (f x))

(defn main []
  (let [keeper (make-keeper)]
    (do
      (churn 256)
      (print (apply keeper 0)))))
