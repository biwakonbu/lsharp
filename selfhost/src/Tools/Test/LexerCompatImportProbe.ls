(module Tools.Test.LexerCompatImportProbe)
(import Syntax.LexerCompat)

(defn sample-spans []
  (let [v0 (vector-new 16)
    v1 (vector-push v0 0)
    v2 (vector-push v1 0)
    v3 (vector-push v2 1)
    v4 (vector-push v3 20)
    v5 (vector-push v4 1)
    v6 (vector-push v5 2)
    v7 (vector-push v6 10)
    v8 (vector-push v7 3)
    v9 (vector-push v8 5)
    v10 (vector-push v9 1)
    v11 (vector-push v10 5)
    v12 (vector-push v11 6)
    v13 (vector-push v12 99)
    v14 (vector-push v13 6)
    v15 (vector-push v14 6)]
    v15))

;; Syntax.LexerCompat 単独 import で legacy/token helper 群が使えることを確かめる。
(defn main []
  (let [legacy "(+)"
    spans-src "(+ 42)"
    kinds (tokenize legacy)
    spans (sample-spans)]
    (do
      (print (vector-length kinds))
      (print (token-count spans))
      (print (token-kind spans 0))
      (print (token-start spans 1))
      (print (token-end spans 1))
      (print (token-int-value spans-src spans 2))
      (print (string-length (token-text spans-src spans 1)))
      0)))
