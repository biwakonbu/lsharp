;; L# 版 fibonacci(35) — ベンチマーク比較用
(defn fib [n]
  (if (<= n 1)
    n
    (+ (fib (- n 1)) (fib (- n 2)))))

(defn main []
  (print (fib 35)))
