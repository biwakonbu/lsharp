;; V4-M1-03-R1 negative boundary: literal record-field patterns are explicit LS3001.
(type Point (record (: x Int) (: y Int)))

(defn main []
  (let [p {Point x 41 y 2}]
    (print
      (match p
        [{Point x 41} 1]
        [_ 0]))))
