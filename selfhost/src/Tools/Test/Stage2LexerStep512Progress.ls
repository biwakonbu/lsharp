(module Tools.Test.Stage2LexerStep512Progress)
(import Syntax.Lexer)

(defn probe-fragment []
  "(defn helper [] 0) ")

(defn build-probe-source-loop [remaining acc]
  (if (<= remaining 0)
    acc
    (build-probe-source-loop
      (- remaining 1)
      (string-concat acc (probe-fragment)))))

(defn build-probe-source []
  (string-concat
    (build-probe-source-loop 36 "")
    "(defn main [] 42)"))

(defn main []
  (let [src (build-probe-source)
    len (string-length src)
    step1 (tokenize-spans-step-512 src 0 len (vector-new 32))
    done1 (vector-get step1 0)
    next1 (vector-get step1 1)
    count1 (token-count (vector-get step1 2))]
    (do
      (print len)
      (print done1)
      (print next1)
      (print count1)
      (if (= done1 1)
        0
        (let [step2 (tokenize-spans-step-512 src next1 len (vector-get step1 2))
          done2 (vector-get step2 0)
          next2 (vector-get step2 1)
          count2 (token-count (vector-get step2 2))]
          (do
            (print done2)
            (print next2)
            (print count2)
            0))))))
