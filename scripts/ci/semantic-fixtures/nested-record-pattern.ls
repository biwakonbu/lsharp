;; V4-M1-03-R1: nested record construction and pattern projection.
(type Inner (record (: x Int)))
(type Outer (record (: inner Inner)))

(defn read-inner [o]
  (match o
    [{Outer inner {Inner x x}} x]
    [_ 0]))

(defn check-literal-value [o]
  (match o
    [{Outer inner {Inner x x}} (if (= x 41) 1 0)]
    [_ 0]))

(defn check-literal-miss [o]
  (match o
    [{Outer inner {Inner x x}} (if (= x 42) 1 7)]
    [_ 0]))

(defn main []
  (let [p {Outer inner {Inner x 41}}]
    (do
      (print (read-inner p))
      (print (check-literal-value p))
      (print (check-literal-miss p))
      0)))
