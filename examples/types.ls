;; 代数的データ型の例
(type (Option a)
  (Some a)
  None)

(type (Result a e)
  (Ok a)
  (Err e))

;; 型注釈付き関数
(defn unwrap-or [(: opt (Option Int)) (: default Int)] : Int
  (match opt
    [(Some x) x]
    [None default]))

(defn main []
  (let [x (Some 42)
        y None]
    (do
      (print (unwrap-or x 0))
      (print (unwrap-or y 0)))))
