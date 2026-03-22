; nested-module.ls - ネストしたモジュールのサンプル

(module Utils)

(defn double [x] (* x 2))
(defn square [x] (* x x))

(defn main []
  (do
    (print (double 5))
    (print (square 4))
    (print (+ (double 3) (square 2)))
    0))
