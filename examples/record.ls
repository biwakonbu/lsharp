; レコード型サンプル
(type Point (record (: x Int) (: y Int)))

(defn make-point [x y]
  {Point x x y y})

(defn get-x [p]
  (Point.x p))

(defn main []
  (let [p (make-point 10 20)]
    (print (get-x p))))
