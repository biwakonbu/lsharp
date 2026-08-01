;; V4-M1-03-R4 positive boundary: an invocation without user arguments
;; observes the deterministic WASI program-name entry at argv[0].
(defn main []
  (print (command-line-args)))
