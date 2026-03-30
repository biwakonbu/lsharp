(module Syntax.LexerCompat)
(import Syntax.Lexer)

;; Syntax.Lexer の compiler-critical path から legacy/test helper を切り離す。

(defn append-kind-token [tokens kind]
  (vector-push tokens kind))

(defn tokenize-step [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (make-tokenize-state 1 ws-pos (append-kind-token tokens 99))
      (let [result (lex-one src ws-pos len)
        kind (/ result 1000000)
        end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (make-tokenize-state 1 ws-pos (append-kind-token tokens 99))
          (make-tokenize-state 0 end-pos (append-kind-token tokens kind)))))))

(defn tokenize-loop-1 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-2 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-2 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-3 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-3 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-4 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-4 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-5 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-5 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-6 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-6 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-7 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-7 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop-8 src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop-8 [src pos len tokens]
  (let [step (tokenize-step src pos len tokens)
    done (vector-get step 0)]
    (if (= done 1)
      (vector-get step 2)
      (tokenize-loop src (vector-get step 1) len (vector-get step 2)))))

(defn tokenize-loop [src pos len tokens]
  (tokenize-loop-1 src pos len tokens))

;; ソース文字列をトークン化して種別の Vector を返す (後方互換)
(defn tokenize [src]
  (tokenize-loop src 0 (string-length src) (vector-new 16)))

(defn token-count [tokens]
  (/ (vector-length tokens) 3))

(defn token-kind [tokens n]
  (vector-get tokens (* n 3)))

(defn token-start [tokens n]
  (vector-get tokens (+ (* n 3) 1)))

(defn token-end [tokens n]
  (vector-get tokens (+ (* n 3) 2)))

(defn token-int-value [src tokens n]
  (let [start (token-start tokens n)
    end (token-end tokens n)]
    (parse-int-from-string src start end 0)))

(defn token-text [src tokens n]
  (substring src (token-start tokens n) (token-end tokens n)))
