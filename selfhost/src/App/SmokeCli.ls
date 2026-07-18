(module App.SmokeCli)
(import Syntax.Lexer)
(import Syntax.Parser)
(import Types.TypeInfer)
(import Backend.Wasm.Compiler)
(import Backend.Wasm.WasmEmit)

(defn exit-success [] 0)
(defn exit-compile-error [] 1)
(defn cmd-parse [] 1)
(defn cmd-check [] 2)
(defn cmd-compile [] 3)
(defn cmd-build [] 4)
(defn cmd-fmt [] 12)
(defn arg-parse [cmd-name] (if (string-eq cmd-name "parse") (cmd-parse) (if (string-eq cmd-name "check") (cmd-check) (if (string-eq cmd-name "compile") (cmd-compile) (if (string-eq cmd-name "build") (cmd-build) (if (string-eq cmd-name "fmt") (cmd-fmt) 0))))))
(defn parse-first-decl-tag [program] (if (> (vector-length program) 0) (vector-get (vector-get program 0) 0) 0))
(defn parse-decl-tag-text [tag] (if (= tag 20) "defn" (if (= tag 25) "module" (if (= tag 26) "import" (string-concat "decl-" (int-to-string tag))))))
(defn parse-expr-tag-text [tag] (if (= tag 1) "int" (if (= tag 2) "bool" (if (= tag 3) "string" (if (= tag 4) "var" (if (= tag 5) "apply" (if (= tag 6) "if" (if (= tag 7) "let" (if (= tag 8) "fn" (if (= tag 9) "do" (if (= tag 10) "match" (if (= tag 32) "unit" (string-concat "expr-" (int-to-string tag))))))))))))))
(defn parse-first-decl-text [program] (if (> (vector-length program) 0) (parse-decl-tag-text (vector-get (vector-get program 0) 0)) "none"))
(defn parse-defn-body-index [decl] (+ 3 (vector-get decl 2)))
(defn parse-first-body-tag [program] (if (> (vector-length program) 0) (let [decl0 (vector-get program 0)] (if (= (vector-get decl0 0) 20) (vector-get (vector-get decl0 (parse-defn-body-index decl0)) 0) 0)) 0))
(defn parse-first-body-text [program] (let [tag (parse-first-body-tag program)] (if (= tag 0) "none" (parse-expr-tag-text tag))))
(defn parse-decl-count-text [program] (string-concat "decls:" (int-to-string (vector-length program))))
(defn diagnostics-summary-text [count code body] (if (= count 0) "diagnostics:0" (string-concat "diagnostics:" (string-concat (int-to-string count) (string-concat "," (string-concat code (string-concat "@1:1" (string-concat ",first-body:" body))))))))
(defn parse-diagnostic-code [diag] (vector-get diag 1))
(defn parse-diagnostics-first-code [diagnostics] (if (> (vector-length diagnostics) 0) (parse-diagnostic-code (vector-get diagnostics 0)) 0))
(defn parse-diagnostic-body-from-code [code] (if (= code 1001) "unexpected token )" (if (= code 1002) "unexpected token ]" "parse error")))
(defn parse-diagnostics-body-text [diagnostics] (if (> (vector-length diagnostics) 0) (parse-diagnostic-body-from-code (parse-diagnostics-first-code diagnostics)) ""))
(defn check-diagnostics-body-text [program] (if (= (check-diagnostics-first-code program) 0) "" "type error"))
(defn run-parse-source [src opts] (let [program (parse-program src) diagnostics (parse-diagnostics src) diagnostics-count (vector-length diagnostics) diagnostics-text (diagnostics-summary-text diagnostics-count "P0001" (parse-diagnostics-body-text diagnostics))] (do (print-string (parse-decl-count-text program)) (print-string "
") (print-string (string-concat "first-decl:" (parse-first-decl-text program))) (print-string "
") (print-string (string-concat "first-body:" (parse-first-body-text program))) (print-string "
") (print-string diagnostics-text) (print-string "
") (exit-success))))
(defn run-check-source [src opts] (let [program (parse-program src) _ (infer program) diagnostics-count (check-diagnostics-count-program program) diagnostics-text (diagnostics-summary-text diagnostics-count "T0001" (check-diagnostics-body-text program))] (do (print-string "check:ok") (print-string "
") (print-string diagnostics-text) (print-string "
") (exit-success))))
(defn run-fmt-source [src opts] (do (print-string src) (exit-success)))
(defn wasm-size-text [size] (string-concat "wasm-size:" (int-to-string size)))
(defn compile-source-wasm-bytes [src] (let [program (parse-program src) pair (compile-program-functions-with-source src program) functions (vector-get pair 1) data (vector-get pair 2)] (build-wasm-bytes-wasi functions data)))
(defn run-compile-source [src opts] (let [wasm-bytes (compile-source-wasm-bytes src) wasm-size (vector-length wasm-bytes)] (do (print-string (wasm-size-text wasm-size)) (print-string "
") (exit-success))))
(defn run-compile-output [file-path output-path] (if (file-exists? file-path) (let [wasm-bytes (compile-source-wasm-bytes (read-file file-path)) summary (wasm-size-text (vector-length wasm-bytes))] (do (write-file-bytes output-path wasm-bytes) (print-string summary) (print-string "
") (exit-success))) (exit-compile-error)))
(defn run-build-output [file-path output-path] (run-compile-output file-path output-path))
(defn run-parse [file-path opts] (if (file-exists? file-path) (run-parse-source (read-file file-path) opts) (exit-compile-error)))
(defn run-check [file-path opts] (if (file-exists? file-path) (run-check-source (read-file file-path) opts) (exit-compile-error)))
(defn run-compile [file-path opts] (if (file-exists? file-path) (run-compile-source (read-file file-path) opts) (exit-compile-error)))
(defn run-build [file-path opts] (if (file-exists? file-path) (run-compile file-path opts) (exit-compile-error)))
(defn run-fmt [file-path opts] (if (file-exists? file-path) (run-fmt-source (read-file file-path) opts) (exit-compile-error)))
(defn parse-diagnostics-loop [spans pos-ref src diagnostics] (if (== (p-current spans pos-ref) 99) diagnostics (let [before (ref-get pos-ref) parsed (parse-with-recovery spans pos-ref src diagnostics) next-diagnostics (vector-get parsed 1)] (if (= (ref-get pos-ref) before) (do (p-advance pos-ref) (parse-diagnostics-loop spans pos-ref src next-diagnostics)) (parse-diagnostics-loop spans pos-ref src next-diagnostics)))))
(defn parse-diagnostics [src]
  (let [spans (tokenize-with-spans src)
    pos-ref (ref-new 0)
    delimiter-diagnostics (parse-delimiter-diagnostics spans src)]
    (if (> (vector-length delimiter-diagnostics) 0)
      delimiter-diagnostics
      (parse-diagnostics-loop spans pos-ref src (collect-diagnostics)))))
(defn check-diagnostics-count-program [program] 0)
(defn check-diagnostics-first-code [program] 0)
(defn output-option-flag [arg] (or (string-eq arg "-o") (string-eq arg "--output")))
(defn run-command [cmd-name file-path opts] (let [cmd-id (arg-parse cmd-name)] (if (= cmd-id (cmd-parse)) (run-parse file-path opts) (if (= cmd-id (cmd-check)) (run-check file-path opts) (if (= cmd-id (cmd-compile)) (run-compile file-path opts) (if (= cmd-id (cmd-build)) (run-build file-path opts) (if (= cmd-id (cmd-fmt)) (run-fmt file-path opts) (exit-compile-error))))))))
(defn main [] (let [argc (command-line-args)] (if (= argc 0) 0 (let [cmd-name (command-line-arg 0) file-path (if (> argc 1) (command-line-arg 1) "") flag (if (> argc 2) (command-line-arg 2) "") output-path (if (> argc 3) (command-line-arg 3) "")] (if (and (> argc 3) (output-option-flag flag)) (if (string-eq cmd-name "compile") (run-compile-output file-path output-path) (if (string-eq cmd-name "build") (run-build-output file-path output-path) (run-command cmd-name file-path 0))) (run-command cmd-name file-path 0))))))
