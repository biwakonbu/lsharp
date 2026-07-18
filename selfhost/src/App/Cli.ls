(module App.Cli)
(import App.ModuleResolver)
(import App.CompilerMode)
(import Syntax.AST)
(import Backend.Wasm.Compiler)
(import Tools.Doc.DocTools)
(import Tools.Doc.DocJson)
(import Tools.Text.Formatter)
(import Tools.Text.FormatterDecl)
(import Syntax.Lexer)
(import Tools.Lsp.JsonRpc)
(import Tools.Lsp.LspServer)
(import Tools.Lsp.LspServerCore)
(import Tools.Lsp.LspServerNav)
(import Syntax.Parser)
(import Tools.Test.TestRunner)
(import Tools.Test.PropertyRunner)
(import Types.TypeInfer)
(import Types.TypeInferCore)
(import Types.TypeScheme)
(import Types.TypeInferAssertions)
(import Types.MetadataMigration)
(import Backend.Wasm.CompilerBase)
(defn push-int-vector-local [dst value] (do (root_push dst) (let [next-dst (vector-push dst value)] (do (root_pop) next-dst))))
(defn push-object-vector-local [dst value] (do (root_push dst) (root_push value) (let [next-dst (vector-push dst value)] (do (root_pop) (root_pop) next-dst))))
(defn exit-success [] 0)
(defn exit-compile-error [] 1)
(defn exit-runtime-error [] 2)
(defn exit-unknown-command [] 127)
(defn exit-code-success [] 0)
(defn exit-code-compile-error [] 1)
(defn exit-code-runtime-error [] 2)
(defn exit-code-unknown-command [] 127)
(defn compile-target-preview1 [] 0)
(defn compile-target-component [] 1)
(defn compile-target-invalid [] (- 0 1))
(defn default-compile-target [] (compile-target-preview1))
(defn parse-compile-target-name [target-name] (if (string-eq target-name "wasi-preview1") (compile-target-preview1) (if (or (string-eq target-name "wasi-component") (string-eq target-name "wasm")) (compile-target-component) (compile-target-invalid))))
(defn cmd-parse [] 1)
(defn cmd-check [] 2)
(defn cmd-compile [] 3)
(defn cmd-build [] 4)
(defn cmd-test [] 5)
(defn cmd-review [] 6)
(defn cmd-doc-ack [] 7)
(defn cmd-doc-check [] 8)
(defn cmd-install [] 9)
(defn cmd-repl [] 10)
(defn cmd-lsp [] 11)
(defn cmd-fmt [] 12)
(defn cmd-doc [] 13)
(defn arg-parse [cmd-name] (if (string-eq cmd-name "parse") (cmd-parse) (if (string-eq cmd-name "check") (cmd-check) (if (string-eq cmd-name "compile") (cmd-compile) (if (string-eq cmd-name "build") (cmd-build) (if (string-eq cmd-name "test") (cmd-test) (if (string-eq cmd-name "review") (cmd-review) (if (string-eq cmd-name "doc-ack") (cmd-doc-ack) (if (string-eq cmd-name "doc-check") (cmd-doc-check) (if (string-eq cmd-name "install") (cmd-install) (if (string-eq cmd-name "repl") (cmd-repl) (if (string-eq cmd-name "lsp") (cmd-lsp) (if (string-eq cmd-name "fmt") (cmd-fmt) (if (string-eq cmd-name "doc") (cmd-doc) 0))))))))))))))
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
(defn check-diagnostic-body-from-code [code] (if (= code (canonical-assertion-type-error-code)) "assert predicate type error" (if (= code (canonical-assertion-non-bool-code)) "assert predicate must be Bool" (if (= code (canonical-assertion-empty-code)) "assert requires at least one predicate" (if (= code (canonical-assertion-vacuous-code)) "assert predicate is vacuous" (if (= code (error-code-undefined)) "undefined symbol" (if (= code (error-code-if-cond)) "if condition must be Bool" (if (= code (error-code-if-branch)) "if branches must have same type" (if (= code (error-code-arg-mismatch)) "function argument type mismatch" (if (= code (error-code-infinite)) "infinite type" "type error"))))))))))
(defn check-case-diagnostic-body-from-code [code] (if (= code (canonical-case-type-error-code)) "case expression type error" (if (= code (canonical-case-value-error-code)) "case actual and expected types must be Int or Bool" (if (= code (canonical-case-empty-code)) "case requires at least one expectation" "case type error"))))
(defn check-property-diagnostic-body-from-code [code] (if (= code (canonical-property-type-error-code)) "property predicate type error" (if (= code (canonical-property-non-bool-code)) "property predicate must be Bool" (if (= code (canonical-property-empty-code)) "property requires typed binders, a postcondition, and positive cases" "property predicate type error"))))
(defn check-diagnostics-body-text [program] (let [code (check-diagnostics-first-code program)] (if (= code 0) "" (check-diagnostic-body-from-code code))))
(defn check-option-json [] 1)
(defn check-json-diagnostics [count first-error-code body]
  (let [fields0 ""
    fields1 (legacy-json-append-field fields0 (legacy-json-int-field "count" count))
    fields2 (legacy-json-append-field fields1 (legacy-json-int-field "firstErrorCode" first-error-code))
    fields3 (legacy-json-append-field fields2 (legacy-json-field "message" body))]
    (string-concat "{" (string-concat fields3 "}"))))
(defn check-json-report [rendered diagnostics-count first-error-code diagnostics-body migration-rows]
  (let [fields0 ""
    fields1 (legacy-json-append-field fields0 (legacy-json-field "command" "check"))
    fields2 (legacy-json-append-field fields1 (legacy-json-field "type" rendered))
    diagnostics (check-json-diagnostics diagnostics-count first-error-code diagnostics-body)
    fields3 (legacy-json-append-field fields2 (string-concat "\"diagnostics\":" diagnostics))
    fields4 (legacy-json-append-field fields3 (string-concat "\"migration\":" (legacy-migration-detail-json-summary migration-rows)))]
    (string-concat "{" (string-concat fields4 "}"))))
(defn run-parse-source [src opts] (let [program (parse-program src) diagnostics (parse-diagnostics src) diagnostics-count (vector-length diagnostics) diagnostics-text (diagnostics-summary-text diagnostics-count "P0001" (parse-diagnostics-body-text diagnostics))] (do (print-string (parse-decl-count-text program)) (print-string "
") (print-string (string-concat "first-decl:" (parse-first-decl-text program))) (print-string "
") (print-string (string-concat "first-body:" (parse-first-body-text program))) (print-string "
") (print-string diagnostics-text) (print-string "
") (exit-success))))
(defn builtin-type-name-text [type-hash] (if (= type-hash 100) "Int" (if (= type-hash 200) "Bool" (if (= type-hash 300) "String" (if (= type-hash 400) "Float" (if (= type-hash 500) "Unit" (string-concat "type-" (int-to-string type-hash))))))))
(defn render-type-text [ty] (let [tag (ty-tag ty)] (if (= tag 1) (builtin-type-name-text (ty-name ty)) (if (= tag 2) (string-concat "t" (int-to-string (ty-name ty))) (if (= tag 3) "Fn" (if (= tag 4) (string-concat "record-" (int-to-string (ty-name ty))) "Unknown"))))))
(defn run-check-source [src opts]
  (let [program (parse-program src)
    analysis (infer-program-analysis program)
    ty (infer-program-analysis-type analysis)
    rendered (render-type-text ty)
    base-diagnostics-count (infer-program-analysis-diagnostic-count analysis)
    base-first-error-code (infer-program-analysis-first-error-code analysis)
    canonical-check (check-canonical-assertions-with-analysis program analysis)
    canonical-diagnostics-count (vector-get canonical-check 0)
    canonical-first-error-code (vector-get canonical-check 1)
    case-check (check-canonical-cases-with-analysis program analysis)
    case-diagnostics-count (vector-get case-check 0)
    property-check (check-canonical-properties-with-analysis program analysis)
    property-diagnostics-count (vector-get property-check 0)
    migration-rows (classify-legacy-contracts program)
    migration-summary (legacy-migration-summary migration-rows)
    migration-detail (legacy-migration-detail-summary migration-rows)
    case-first-error-code (vector-get case-check 1)
    property-first-error-code (vector-get property-check 1)
    diagnostics-count (+ base-diagnostics-count (+ canonical-diagnostics-count (+ case-diagnostics-count property-diagnostics-count)))
    first-error-code
      (if (= base-first-error-code 0)
        (if (= canonical-first-error-code 0)
          (if (= case-first-error-code 0) property-first-error-code case-first-error-code)
          canonical-first-error-code)
        base-first-error-code)
    diagnostics-body
      (if (> base-first-error-code 0)
        (check-diagnostic-body-from-code base-first-error-code)
        (if (> canonical-first-error-code 0)
          (check-diagnostic-body-from-code canonical-first-error-code)
          (if (> case-first-error-code 0)
            (check-case-diagnostic-body-from-code case-first-error-code)
            (if (> property-first-error-code 0)
              (check-property-diagnostic-body-from-code property-first-error-code)
              ""))))
    diagnostics-text (diagnostics-summary-text diagnostics-count "T0001" diagnostics-body)]
    (if (= opts (check-option-json))
      (do
        (print-string (check-json-report rendered diagnostics-count first-error-code diagnostics-body migration-rows))
        (print-string "\n")
        (exit-success))
      (do
      (print-string rendered)
      (print-string "\n")
      (print-string diagnostics-text)
      (print-string "\n")
      (if (> (string-length migration-summary) 0)
        (do
          (print-string migration-summary)
          (print-string "\n"))
        (print-string ""))
      (if (> (string-length migration-detail) 0)
        (do
          (print-string migration-detail)
          (print-string "\n"))
        (print-string ""))
      (exit-success)))))
(defn run-fmt-source [src opts] (let [program (parse-program src) formatted (format-program-with-source program src)] (do (print-string formatted) (exit-success))))
(defn wasm-size-text [size] (string-concat "wasm-size:" (int-to-string size)))
(defn compile-file-functions-data-with-cache [file-path cache-ref parse-count-ref] (compile-file-functions-payload-with-cache file-path 12 cache-ref parse-count-ref))
(defn compile-file-functions-data [file-path] (let [cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0)] (compile-file-functions-data-with-cache file-path cache-ref parse-count-ref)))
(defn standalone-preview1-capability-boundary-message [] "unsupported standalone Preview1 runtime capability")
(defn standalone-preview1-input-layout-safe? [src] (< (string-length src) 1024))
(defn standalone-preview1-data-layout-safe? [data] (< (vector-length data) (standalone-data-layout-limit)))
(defn compile-file-wasm-bytes [file-path] (let [pair (compile-file-functions-data file-path) functions (vector-get pair 0) data (vector-get pair 1) unsupported-opcode (standalone-preview1-first-unsupported-opcode functions)] (if (>= unsupported-opcode 0) (vector-new 0) (if (standalone-preview1-data-layout-safe? data) (build-wasm-bytes-wasi-standalone functions data) (vector-new 0)))))
(defn compile-file-wasm-size [file-path target] (vector-length (compile-file-wasm-bytes file-path)))
(defn compile-source-wasm-bytes [src] (let [program (parse-program src) pair (compile-program-functions-with-source-base src program 12) functions (vector-get pair 1) data (vector-get pair 2) unsupported-opcode (standalone-preview1-first-unsupported-opcode functions)] (if (>= unsupported-opcode 0) (vector-new 0) (if (standalone-preview1-data-layout-safe? data) (build-wasm-bytes-wasi-standalone functions data) (vector-new 0)))))
(defn run-compile-source [src opts] (if (standalone-preview1-input-layout-safe? src) (let [wasm-bytes (compile-source-wasm-bytes src) wasm-size (vector-length wasm-bytes)] (if (= wasm-size 0) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error)) (do (print-string (wasm-size-text wasm-size)) (print-string "\n") (exit-success)))) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error))))
(defn component-output-boundary-message [] "wasi-component output requires external component packaging")
(defn run-compile-output [file-path output-path opts]
  (if (file-exists? file-path)
    (if (= opts (compile-target-preview1))
      (let [src (read-file file-path)]
        (if (standalone-preview1-input-layout-safe? src)
          (let [wasm-bytes (compile-file-wasm-bytes file-path)
            wasm-size (vector-length wasm-bytes)
            summary (wasm-size-text wasm-size)]
            (if (= wasm-size 0)
              (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error))
              (do
                (write-file-bytes output-path wasm-bytes)
                (print-string summary)
                (print-string "\n")
                (exit-success))))
          (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error))))
      (do (cli-stderr (component-output-boundary-message)) (exit-compile-error)))
    (exit-compile-error)))
(defn run-build-output [file-path output-path opts] (run-compile-output file-path output-path opts))
(defn test-examples-text [count] (string-concat "examples:" (int-to-string count)))
(defn test-invariants-text [count] (string-concat "invariants:" (int-to-string count)))
(defn test-assertions-text [count] (string-concat "assertions:" (int-to-string count)))
(defn test-cases-text [count] (string-concat "cases:" (int-to-string count)))
(defn test-properties-text [count] (string-concat "properties:" (int-to-string count)))
(defn test-failures-text [count] (string-concat "failures:" (int-to-string count)))
(defn case-preflight-diagnostics-summary [case-check]
  (let [count (vector-get case-check 0)
    raw-code (vector-get case-check 1)
    code (if (= raw-code (canonical-case-type-error-code))
      (contract-diagnostic-undefined)
      (if (= raw-code (canonical-case-value-error-code))
        (contract-diagnostic-non-bool)
        raw-code))]
    (if (= count 0)
      "diagnostics:0"
      (string-concat "diagnostics:"
        (string-concat (int-to-string count)
          (string-concat "," (test-diagnostic-code-text code)))))))
(defn run-test-source-case-preflight [program case-check]
  (let [examples (extract-examples-from-program program)
    invariants (extract-invariants-from-program program)
    assertions (extract-assertions-from-program program)
    cases (extract-cases-from-program program)
    properties (extract-property-test-cases program)
    diagnostic-count (vector-get case-check 0)
    diagnostic-summary (case-preflight-diagnostics-summary case-check)]
    (do
      (print-string (test-examples-text (vector-length examples)))
      (print-string "\n")
      (print-string (test-invariants-text (vector-length invariants)))
      (print-string "\n")
      (if (> (vector-length assertions) 0)
        (do
          (print-string (test-assertions-text (vector-length assertions)))
          (print-string "\n"))
        (print-string ""))
      (if (> (vector-length cases) 0)
        (do
          (print-string (test-cases-text (vector-length cases)))
          (print-string "\n"))
        (print-string ""))
      (if (> (vector-length properties) 0)
        (do
          (print-string (test-properties-text (vector-length properties)))
          (print-string "\n"))
        (print-string ""))
      (print-string (test-failures-text diagnostic-count))
      (print-string "\n")
      (print-string diagnostic-summary)
      (print-string "\n")
      (exit-runtime-error))))
(defn run-test-source [src opts]
  (let [program (parse-program src)
    analysis (infer-program-analysis program)
    property-boundary-code (metadata-test-runner-boundary-code program)
    case-check (check-canonical-cases-with-analysis program analysis)
    case-diagnostics-count (vector-get case-check 0)]
    (if (> property-boundary-code 0)
      (run-test-source-case-preflight
        program
        (vector-push
          (vector-push (vector-new 2) 1)
          property-boundary-code))
      (if (> case-diagnostics-count 0)
        (run-test-source-case-preflight program case-check)
        (let [suite (generate-tests-from-source src)
    example-results (vector-get suite 0)
    invariant-results (vector-get suite 1)
    assertion-results (vector-get suite 2)
    case-results (vector-get suite 3)
    property-results (vector-get suite 4)
    example-count (vector-length example-results)
    invariant-count (vector-length invariant-results)
    assertion-count (vector-length assertion-results)
    case-count (vector-length case-results)
    property-count (vector-length property-results)
    failed (+
      (count-failed-results example-results)
      (+
        (count-failed-results invariant-results)
        (+
          (count-failed-results assertion-results)
          (+ (count-failed-results case-results) (count-failed-results property-results)))))
    diagnostic-count (test-diagnostics-count-with-properties
      example-results
      invariant-results
      assertion-results
      case-results
      property-results)
    diagnostic-summary (test-diagnostics-summary-with-properties
      example-results
      invariant-results
      assertion-results
      case-results
      property-results)]
    (do
      (print-string (test-examples-text example-count))
      (print-string "\n")
      (print-string (test-invariants-text invariant-count))
      (print-string "\n")
      (if (> assertion-count 0)
        (do
          (print-string (test-assertions-text assertion-count))
          (print-string "\n"))
        (print-string ""))
      (if (> case-count 0)
        (do
          (print-string (test-cases-text case-count))
          (print-string "\n"))
        (print-string ""))
      (if (> property-count 0)
        (do
          (print-string (test-properties-text property-count))
          (print-string "\n"))
        (print-string ""))
      (print-string (test-failures-text failed))
      (print-string "\n")
      (if (> diagnostic-count 0)
        (do
          (print-string diagnostic-summary)
          (print-string "\n"))
        (print-string ""))
      (if (> failed 0) (exit-runtime-error) (exit-success))))))))
(defn review-option-json [] 1)
(defn review-json-source-id [] 200)
(defn run-review-source [src opts] (let [program (parse-program src)] (if (= opts (review-option-json)) (let [review-json (generate-review-schema-json program (review-json-source-id))] (do (print-string review-json) (print-string "
") (exit-success))) (let [review (generate-review program opts) diagnostics (vector-get review 1) review-title (review-summary-title diagnostics) review-body (review-summary-body diagnostics) review-severity (review-summary-severity diagnostics) review-code-location (review-summary-code-location diagnostics)] (do (print (vector-length diagnostics)) (print-string review-title) (print-string "
") (print-string review-body) (print-string "
") (print-string review-severity) (print-string "
") (print-string review-code-location) (print-string "
") (exit-success))))))
(defn doc-option-json [] 1)
(defn doc-json-module-id [] 42)
(defn run-doc-source [src opts] (let [program (parse-program src)] (if (= opts (doc-option-json)) (let [doc-json (generate-doc-output-schema-json program (doc-json-module-id))] (do (print-string doc-json) (print-string "
") (exit-success))) (let [doc (generate program opts) title (vector-get doc 0) body (vector-get doc 1)] (do (print-string title) (print-string "
") (print-string body) (print-string "
") (exit-success))))))
(defn run-parse [file-path opts] (if (file-exists? file-path) (run-parse-source (read-file file-path) opts) (exit-compile-error)))
(defn run-check [file-path opts] (if (file-exists? file-path) (run-check-source (read-file file-path) opts) (exit-compile-error)))
(defn run-compile [file-path opts] (if (file-exists? file-path) (if (= opts (compile-target-preview1)) (let [wasm-size (compile-file-wasm-size file-path opts)] (if (= wasm-size 0) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error)) (do (print-string (wasm-size-text wasm-size)) (print-string "
") (exit-success)))) (do (cli-stderr (component-output-boundary-message)) (exit-compile-error))) (exit-compile-error)))
(defn run-build [file-path opts] (if (file-exists? file-path) (run-compile file-path opts) (exit-compile-error)))
(defn run-test [file-path opts] (if (file-exists? file-path) (run-test-source (read-file file-path) opts) (exit-compile-error)))
(defn run-review [file-path opts] (if (file-exists? file-path) (run-review-source (read-file file-path) opts) (exit-compile-error)))
(defn print-doc-trailers-loop [trailers idx count] (if (>= idx count) 0 (do (print-string (vector-get trailers idx)) (print-string "
") (print-doc-trailers-loop trailers (+ idx 1) count))))
(defn print-doc-payload [payload] (let [trailers (vector-get payload 3)] (do (print-string (vector-get payload 0)) (print-string "
") (print-string (vector-get payload 1)) (print-string "
") (print-string (vector-get payload 2)) (print-string "
") (print-doc-trailers-loop trailers 0 (vector-length trailers)))))
(defn print-doc-trailer-only [payload] (let [trailers (vector-get payload 3)] (print-doc-trailers-loop trailers 0 (vector-length trailers))))
(defn doc-option-trailer-only [] 1)
(defn doc-option-strict-check [] 1)
(defn run-doc-ack [file-path opts] (if (file-exists? file-path) (let [src (read-file file-path) program (parse-program src) ack (generate-doc-ack program "anonymous")] (do (if (= opts (doc-option-trailer-only)) (print-doc-trailer-only ack) (print-doc-payload ack)) (exit-success))) (exit-compile-error)))
(defn invalid-doc-trailer-message [] "invalid doc trailer: expected trailing comment lines")
(defn run-doc-check [file-path opts] (if (file-exists? file-path) (let [src (read-file file-path)] (if (and (= opts (doc-option-strict-check)) (= (doc-check-trailer-valid? src) 0)) (do (cli-stderr (invalid-doc-trailer-message)) (exit-compile-error)) (let [program (parse-program src) check (generate-doc-check program "anonymous")] (do (print-doc-payload check) (exit-success))))) (exit-compile-error)))
(defn install-plan-title [package] (string-concat "package:" package))
(defn install-plan-body [package] "status:planned")
(defn run-install [package opts] (if (> (string-length package) 0) (do (print-string (install-plan-title package)) (print-string "
") (print-string (install-plan-body package)) (print-string "
") (exit-success)) (exit-compile-error)))
(defn repl-session-new [] (push-object-vector-local (push-object-vector-local (push-object-vector-local (vector-new 3) (ref-new 0)) (ref-new 0)) (ref-new 0)))
(defn repl-session-eval-count [session] (ref-get (vector-get session 0)))
(defn repl-session-last-type-name [session] (ref-get (vector-get session 1)))
(defn repl-session-total-input-bytes [session] (ref-get (vector-get session 2)))
(defn repl-session-eval [session src] (let [program (parse-program src) ty (infer program) type-name (ty-name ty)] (do (ref-set (vector-get session 0) (+ (repl-session-eval-count session) 1)) (ref-set (vector-get session 1) type-name) (ref-set (vector-get session 2) (+ (repl-session-total-input-bytes session) (string-length src))) type-name)))
(defn repl-session-run-loop [session inputs idx count] (if (>= idx count) 0 (do (repl-session-eval session (vector-get inputs idx)) (repl-session-run-loop session inputs (+ idx 1) count))))
(defn repl-session-run [inputs] (let [session (repl-session-new) _ (repl-session-run-loop session inputs 0 (vector-length inputs)) summary (vector-new 3)] (push-int-vector-local (push-int-vector-local (push-int-vector-local summary (repl-session-eval-count session)) (repl-session-total-input-bytes session)) (repl-session-last-type-name session))))
(defn repl-summary-type-text [summary] (string-concat "type:" (builtin-type-name-text (vector-get summary 2))))
(defn repl-summary-evals-text [summary] (string-concat "evals:" (int-to-string (vector-get summary 0))))
(defn repl-summary-input-bytes-text [summary] (string-concat "input-bytes:" (int-to-string (vector-get summary 1))))
(defn repl-warmup-summary [] (let [inputs (push-object-vector-local (vector-new 1) "(defn main [] 42)")] (repl-session-run inputs)))
(defn repl-warmup-type-name [] (let [summary (repl-warmup-summary)] (vector-get summary 2)))
(defn repl-warmup-type-text [] (builtin-type-name-text (repl-warmup-type-name)))
(defn run-repl [opts] (let [summary (repl-warmup-summary)] (do (print-string (repl-summary-type-text summary)) (print-string "
") (print-string (repl-summary-evals-text summary)) (print-string "
") (print-string (repl-summary-input-bytes-text summary)) (print-string "
") (exit-success))))
(defn lsp-bool-text [value] (if (= value 1) "true" "false"))
(defn lsp-sync-kind-text [kind] (if (= kind 1) "full" (string-concat "sync-" (int-to-string kind))))
(defn lsp-loop-request [method-id params] (push-object-vector-local (push-int-vector-local (vector-new 2) method-id) params))
(defn lsp-init-summary [] (let [requests (push-object-vector-local (vector-new 1) (lsp-loop-request (lsp-method-initialize) 0)) summary (server-loop-sequence requests)] summary))
(defn lsp-init-capabilities [summary] (let [results (vector-get summary 0)] (vector-get results 0)))
(defn lsp-summary-requests-text [summary] (string-concat "requests:" (int-to-string (vector-get summary 2))))
(defn lsp-summary-documents-text [summary] (string-concat "documents:" (int-to-string (vector-get summary 1))))
(defn lsp-summary-source-bytes-text [summary] (string-concat "source-bytes:" (int-to-string (vector-get summary 3))))
(defn lsp-transport-request-id [request] (vector-get request 1))
(defn lsp-transport-method-id [request] (vector-get request 2))
(defn lsp-transport-params [request] (vector-get request 3))
(defn lsp-transport-uri [request] (if (> (vector-length (lsp-transport-params request)) 0) (vector-get (lsp-transport-params request) 0) 0))
(defn lsp-transport-invalid-request-code [] (- 0 32600))
(defn lsp-transport-method-not-found-code [] (- 0 32601))
(defn lsp-transport-request-after-shutdown? [state method-id]
  (if (= (server-state-shutdown state) 1)
    (if (= method-id (lsp-method-shutdown)) 0 1)
    0))
(defn lsp-transport-document-method? [method-id] (if (= method-id (lsp-method-did-open)) 1 (if (= method-id (lsp-method-did-change)) 1 0)))
(defn lsp-diagnostic-source-parse [] 1)
(defn lsp-diagnostic-source-type [] 2)
(defn lsp-diagnostic-source-lint [] 3)
(defn lsp-parser-severity-to-lsp [severity] (+ severity 1))
(defn lsp-parse-diagnostic-to-lsp [diag src]
  (let [position (lsp-position-from-offset src (vector-get diag 2))
    result (vector-new 6)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local
              (push-int-vector-local result (lsp-parser-severity-to-lsp (vector-get diag 0)))
              (vector-get diag 1))
            (position-line position))
          (position-col position))
        (vector-get diag 3))
      (lsp-diagnostic-source-parse))))
(defn lsp-source-parse-diagnostics-loop [raw src idx count diagnostics]
  (if (>= idx count)
    diagnostics
    (lsp-source-parse-diagnostics-loop
      raw
      src
      (+ idx 1)
      count
      (push-object-vector-local diagnostics (lsp-parse-diagnostic-to-lsp (vector-get raw idx) src)))))
(defn lsp-source-parse-diagnostics [src]
  (if (> (string-length src) 0)
    (let [raw (parse-diagnostics src)
      diagnostics (lsp-source-parse-diagnostics-loop raw src 0 (vector-length raw) (vector-new 4))]
      (dedup-diagnostics (sort-diagnostics diagnostics)))
    (vector-new 0)))
(defn lsp-type-severity-to-lsp [] 1)
(defn lsp-type-diagnostic-to-lsp [code]
  (let [result (vector-new 6)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local
              (push-int-vector-local result (lsp-type-severity-to-lsp))
              code)
            1)
          1)
        code)
      (lsp-diagnostic-source-type))))
(defn lsp-source-type-diagnostics [src]
  (if (> (string-length src) 0)
    (let [program (parse-program src)
      code (check-diagnostics-first-code program)]
      (if (= code 0)
        (vector-new 0)
        (push-object-vector-local (vector-new 1) (lsp-type-diagnostic-to-lsp code))))
    (vector-new 0)))
(defn lsp-review-severity-to-lsp [severity]
  (if (string-eq severity "warning")
    2
    (if (string-eq severity "info")
      3
      (if (string-eq severity "hint") 4 1))))
(defn lsp-review-diagnostic-to-lsp [diag]
  (let [result (vector-new 6)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local
              (push-int-vector-local result (lsp-review-severity-to-lsp (vector-get diag 3)))
              (vector-get diag 0))
            (vector-get diag 4))
          (vector-get diag 5))
        (vector-get diag 0))
      (lsp-diagnostic-source-lint))))
(defn lsp-source-lint-diagnostics-loop [raw idx count diagnostics]
  (if (>= idx count)
    diagnostics
    (lsp-source-lint-diagnostics-loop raw (+ idx 1) count (push-object-vector-local diagnostics (lsp-review-diagnostic-to-lsp (vector-get raw idx))))))
(defn lsp-source-lint-diagnostics [src]
  (if (> (string-length src) 0)
    (let [program (parse-program src)
      review (generate-review program 0)
      raw (vector-get review 1)
      diagnostics (lsp-source-lint-diagnostics-loop raw 0 (vector-length raw) (vector-new 4))]
      (dedup-diagnostics (sort-diagnostics diagnostics)))
    (vector-new 0)))
(defn lsp-diagnostics-append-loop [extra idx count diagnostics]
  (if (>= idx count)
    diagnostics
    (lsp-diagnostics-append-loop extra (+ idx 1) count (push-object-vector-local diagnostics (vector-get extra idx)))))
(defn lsp-diagnostics-append [diagnostics extra] (lsp-diagnostics-append-loop extra 0 (vector-length extra) diagnostics))
(defn lsp-source-all-diagnostics [src]
  (if (> (string-length src) 0)
    (let [parse-diagnostics (lsp-source-parse-diagnostics src)]
      (if (> (vector-length parse-diagnostics) 0)
        parse-diagnostics
        (let [type-diagnostics (lsp-source-type-diagnostics src)
          lint-diagnostics (lsp-source-lint-diagnostics src)
          diagnostics (lsp-diagnostics-append (lsp-diagnostics-append (vector-new 8) type-diagnostics) lint-diagnostics)]
          (dedup-diagnostics (sort-diagnostics diagnostics)))))
    (vector-new 0)))
(defn lsp-transport-maybe-append-diagnostics-frame [state method-id uri previous-src rendered]
  (if (= (lsp-transport-document-method? method-id) 1)
    (let [current-src (server-state-source-for-uri state uri)
      previous-diagnostics (lsp-source-all-diagnostics previous-src)
      current-diagnostics (lsp-source-all-diagnostics current-src)]
      (if (> (+ (vector-length previous-diagnostics) (vector-length current-diagnostics)) 0)
        (string-concat rendered (lsp-render-publish-diagnostics-frame uri current-diagnostics))
        rendered))
    rendered))
(defn lsp-transport-dispatch-request [state request]
  (let [request-id (lsp-transport-request-id request)
    method-id (lsp-transport-method-id request)
    params (lsp-transport-params request)
    uri (lsp-transport-uri request)
    reject-after-shutdown (lsp-transport-request-after-shutdown? state method-id)
    previous-src (if (= (lsp-transport-document-method? method-id) 1) (server-state-source-for-uri state uri) "")
    result (if (= reject-after-shutdown 1) 0 (json-rpc-dispatch method-id params state))
    rendered
    (if (= reject-after-shutdown 1)
      (lsp-render-error-frame request-id (lsp-transport-invalid-request-code) "Invalid Request")
      (if (= method-id (lsp-method-initialize))
        (lsp-render-initialize-frame request-id)
        (if (= method-id (lsp-method-shutdown))
          (lsp-render-shutdown-frame request-id)
          (if (= method-id (lsp-method-did-open))
            (lsp-render-didopen-frame uri (server-state-source-length state))
            (if (= method-id (lsp-method-did-change))
              (lsp-render-didchange-frame uri (server-state-source-length state))
              (if (= method-id (lsp-method-goto-def))
                (lsp-render-location-frame request-id result)
                (if (= method-id (lsp-method-hover))
                  (lsp-render-hover-frame request-id result)
                  (if (= method-id (lsp-method-references))
                    (lsp-render-locations-frame request-id result)
                    (if (= method-id (lsp-method-completion))
                      (lsp-render-completion-frame request-id result)
                      (if (= method-id (lsp-method-formatting))
                        (lsp-render-formatting-frame request-id result)
                        (if (= method-id (lsp-method-rename))
                          (lsp-render-rename-frame request-id result)
                          (if (= method-id (lsp-method-publish-diagnostics))
                            (lsp-render-publish-diagnostics-frame uri (vector-get params 1))
                            (lsp-render-error-frame request-id (lsp-transport-method-not-found-code) "Method not found")))))))))))))]
    (if (= reject-after-shutdown 1)
      rendered
      (lsp-transport-maybe-append-diagnostics-frame state method-id uri previous-src rendered))))
(defn run-lsp-transport-request [request] (let [state (server-state-new)] (lsp-transport-dispatch-request state request)))
(defn lsp-transport-sequence-loop [state requests idx count frames] (if (>= idx count) frames (lsp-transport-sequence-loop state requests (+ idx 1) count (push-object-vector-local frames (lsp-transport-dispatch-request state (vector-get requests idx))))))
(defn run-lsp-transport-sequence [requests] (let [state (server-state-new) frames (lsp-transport-sequence-loop state requests 0 (vector-length requests) (vector-new 8)) summary (vector-new 4)] (push-int-vector-local (push-int-vector-local (push-int-vector-local (push-object-vector-local summary frames) (server-state-doc-count state)) (server-state-request-count state)) (server-state-source-length state))))
(defn lsp-stdio-frame-header [frame] (vector-get frame 0))
(defn lsp-stdio-frame-message [frame] (vector-get frame 1))
(defn lsp-stdio-frame-content-length [frame] (lsp-parse-content-length (lsp-stdio-frame-header frame)))
(defn lsp-stdio-message-request [msg] (let [parsed (parse-json-rpc-request msg) request-id (vector-get msg 1) method-id (vector-get parsed 0) params (vector-get parsed 1)] (push-object-vector-local (push-int-vector-local (push-int-vector-local (push-int-vector-local (vector-new 4) 2) request-id) method-id) params)))
(defn lsp-stdio-dispatch-frame [state frame] (let [request (lsp-stdio-message-request (lsp-stdio-frame-message frame)) rendered (lsp-transport-dispatch-request state request) content-length (lsp-stdio-frame-content-length frame)] (push-int-vector-local (push-object-vector-local (vector-new 2) rendered) content-length)))
(defn run-lsp-stdio-frame [frame] (let [state (server-state-new)] (lsp-stdio-dispatch-frame state frame)))
(defn lsp-stdio-sequence-loop [state frames idx count rendered last-content-length] (if (>= idx count) (push-int-vector-local (push-object-vector-local (vector-new 2) rendered) last-content-length) (let [result (lsp-stdio-dispatch-frame state (vector-get frames idx))] (lsp-stdio-sequence-loop state frames (+ idx 1) count (push-object-vector-local rendered (vector-get result 0)) (vector-get result 1)))))
(defn run-lsp-stdio-sequence [frames] (let [state (server-state-new) result (lsp-stdio-sequence-loop state frames 0 (vector-length frames) (vector-new 8) 0) rendered (vector-get result 0) last-content-length (vector-get result 1) summary (vector-new 4)] (push-int-vector-local (push-int-vector-local (push-int-vector-local (push-object-vector-local summary rendered) (server-state-request-count state)) (server-state-source-length state)) last-content-length)))
(defn lsp-stdio-find-header-end-loop [src idx len] (if (> (+ idx 3) len) len (if (= (string-char-at src idx) 13) (if (= (string-char-at src (+ idx 1)) 10) (if (= (string-char-at src (+ idx 2)) 13) (if (= (string-char-at src (+ idx 3)) 10) idx (lsp-stdio-find-header-end-loop src (+ idx 1) len)) (lsp-stdio-find-header-end-loop src (+ idx 1) len)) (lsp-stdio-find-header-end-loop src (+ idx 1) len)) (lsp-stdio-find-header-end-loop src (+ idx 1) len))))
(defn lsp-stdio-find-pattern-loop [src pattern idx len]
  (if (>= idx len)
    (- 0 1)
    (if (lsp-match-at src idx pattern)
      idx
      (lsp-stdio-find-pattern-loop src pattern (+ idx 1) len))))

(defn lsp-stdio-is-digit [c]
  (if (< c 48) false (if (> c 57) false true)))

(defn lsp-stdio-parse-int-loop [src idx len acc started]
  (if (>= idx len)
    acc
    (let [c (string-char-at src idx)]
      (if (lsp-stdio-is-digit c)
        (lsp-stdio-parse-int-loop src (+ idx 1) len (+ (- c 48) (* acc 10)) 1)
        (if (= started 1)
          acc
          (lsp-stdio-parse-int-loop src (+ idx 1) len acc 0))))))

(defn lsp-stdio-find-string-end-escaped-loop [src idx len escaped]
  (if (>= idx len)
    len
    (let [ch (string-char-at src idx)]
      (if (= escaped 1)
        (lsp-stdio-find-string-end-escaped-loop src (+ idx 1) len 0)
        (if (= ch 92)
          (lsp-stdio-find-string-end-escaped-loop src (+ idx 1) len 1)
          (if (= ch 34)
            idx
            (lsp-stdio-find-string-end-escaped-loop src (+ idx 1) len 0)))))))

(defn lsp-stdio-find-string-end-loop [src idx len]
  (lsp-stdio-find-string-end-escaped-loop src idx len 0))

(defn lsp-stdio-hex-digit-value [digit]
  (if (>= digit 48)
    (if (<= digit 57)
      (- digit 48)
      (if (>= digit 65)
        (if (<= digit 70)
          (+ 10 (- digit 65))
          (if (>= digit 97)
            (if (<= digit 102)
              (+ 10 (- digit 97))
              (- 0 1))
            (- 0 1)))
        (if (>= digit 97)
          (if (<= digit 102)
            (+ 10 (- digit 97))
            (- 0 1))
          (- 0 1))))
    (- 0 1)))

(defn lsp-stdio-json-printable-ascii []
  " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~")

(defn lsp-stdio-json-unicode-code [src idx len]
  (if (>= (+ idx 5) len)
    (- 0 1)
    (let [d1 (lsp-stdio-hex-digit-value (string-char-at src (+ idx 2)))
      d2 (lsp-stdio-hex-digit-value (string-char-at src (+ idx 3)))
      d3 (lsp-stdio-hex-digit-value (string-char-at src (+ idx 4)))
      d4 (lsp-stdio-hex-digit-value (string-char-at src (+ idx 5)))]
      (if (>= d1 0)
        (if (>= d2 0)
          (if (>= d3 0)
            (if (>= d4 0)
              (+ d4 (* 16 (+ d3 (* 16 (+ d2 (* 16 d1))))))
              (- 0 1))
            (- 0 1))
          (- 0 1))
        (- 0 1)))))

(defn lsp-stdio-json-unicode-piece [src idx len]
  (if (>= (+ idx 5) len)
    (substring src idx (+ idx 2))
    (let [code (lsp-stdio-json-unicode-code src idx len)]
      (if (< code 0)
        (substring src idx (+ idx 6))
        (if (= code 10)
          "\n"
          (if (= code 13)
            "\r"
            (if (= code 9)
              "\t"
              (if (>= code 32)
                (if (<= code 126)
                  (let [ascii (lsp-stdio-json-printable-ascii)
                    ascii-idx (- code 32)]
                    (substring ascii ascii-idx (+ ascii-idx 1)))
                  (substring src idx (+ idx 6)))
                (substring src idx (+ idx 6))))))))))

(defn lsp-stdio-json-unescape-consumed [src idx len]
  (if (>= (+ idx 1) len)
    1
    (let [escaped (string-char-at src (+ idx 1))]
      (if (= escaped 117)
        (if (>= (+ idx 5) len) 2 6)
        2))))

(defn lsp-stdio-json-unescape-piece [src idx len]
  (if (>= (+ idx 1) len)
    "\\"
    (let [escaped (string-char-at src (+ idx 1))]
      (if (= escaped 34)
        "\""
        (if (= escaped 92)
          "\\"
          (if (= escaped 47)
            "/"
            (if (= escaped 110)
              "\n"
              (if (= escaped 114)
                "\r"
                (if (= escaped 116)
                  "\t"
                  (if (= escaped 117)
                    (lsp-stdio-json-unicode-piece src idx len)
                    (substring src (+ idx 1) (+ idx 2))))))))))))

(defn lsp-stdio-json-unescape-loop [src idx end out]
  (if (>= idx end)
    out
    (let [ch (string-char-at src idx)]
      (if (= ch 92)
        (lsp-stdio-json-unescape-loop
          src
          (+ idx (lsp-stdio-json-unescape-consumed src idx end))
          end
          (string-concat out (lsp-stdio-json-unescape-piece src idx end)))
        (lsp-stdio-json-unescape-loop
          src
          (+ idx 1)
          end
          (string-concat out (substring src idx (+ idx 1))))))))

(defn lsp-stdio-json-unescape [src start end]
  (lsp-stdio-json-unescape-loop src start end ""))

(defn lsp-stdio-body-has-field [body pattern]
  (if (>= (lsp-stdio-find-pattern-loop body pattern 0 (string-length body)) 0) 1 0))

(defn lsp-stdio-body-int-field [body pattern]
  (let [len (string-length body)
    pos (lsp-stdio-find-pattern-loop body pattern 0 len)]
    (if (< pos 0)
      0
      (lsp-stdio-parse-int-loop body (+ pos (string-length pattern)) len 0 0))))

(defn lsp-stdio-body-int-field-or [body primary-pattern fallback-pattern]
  (if (= (lsp-stdio-body-has-field body primary-pattern) 1)
    (lsp-stdio-body-int-field body primary-pattern)
    (lsp-stdio-body-int-field body fallback-pattern)))

(defn lsp-stdio-body-int-field-from [body pattern start]
  (let [len (string-length body)
    pos (lsp-stdio-find-pattern-loop body pattern start len)]
    (if (< pos 0)
      0
      (lsp-stdio-parse-int-loop body (+ pos (string-length pattern)) len 0 0))))

(defn lsp-stdio-body-string-field [body pattern]
  (let [len (string-length body)
    pos (lsp-stdio-find-pattern-loop body pattern 0 len)]
    (if (< pos 0)
      ""
      (let [start (+ pos (string-length pattern))
        end (lsp-stdio-find-string-end-loop body (+ pos (string-length pattern)) len)]
        (lsp-stdio-json-unescape body start end)))))

(defn lsp-stdio-body-id [body]
  (let [len (string-length body)
    id-pos (lsp-stdio-find-pattern-loop body "\"id\":" 0 len)]
    (if (< id-pos 0)
      0
      (lsp-stdio-parse-int-loop body (+ id-pos 5) len 0 0))))

(defn lsp-stdio-body-method [body]
  (let [len (string-length body)]
    (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"initialize\"" 0 len) 0)
      (lsp-method-initialize)
      (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"shutdown\"" 0 len) 0)
        (lsp-method-shutdown)
        (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/definition\"" 0 len) 0)
          (lsp-method-goto-def)
          (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/hover\"" 0 len) 0)
            (lsp-method-hover)
            (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/references\"" 0 len) 0)
              (lsp-method-references)
              (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/formatting\"" 0 len) 0)
                (lsp-method-formatting)
                (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/rename\"" 0 len) 0)
                  (lsp-method-rename)
                  (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/didOpen\"" 0 len) 0)
                    (lsp-method-did-open)
                    (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/didChange\"" 0 len) 0)
                      (lsp-method-did-change)
                      (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/completion\"" 0 len) 0)
                        (lsp-method-completion)
                        (if (>= (lsp-stdio-find-pattern-loop body "\"method\":\"textDocument/publishDiagnostics\"" 0 len) 0)
                          (lsp-method-publish-diagnostics)
                          999)))))))))))))

(defn lsp-stdio-nav-params [body]
  (let [params (vector-new 4)
    with-position
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local params (lsp-stdio-body-int-field body "\"uri\":"))
        (lsp-stdio-body-int-field body "\"line\":"))
      (lsp-stdio-body-int-field-or body "\"col\":" "\"character\":"))]
    (if (= (lsp-stdio-body-has-field body "\"source\":\"") 1)
      (push-object-vector-local with-position (lsp-stdio-body-string-field body "\"source\":\""))
      with-position)))

(defn lsp-stdio-document-params [body]
  (let [with-uri
    (push-int-vector-local (vector-new 2) (lsp-stdio-body-int-field body "\"uri\":"))]
    (let [with-source
      (if (= (lsp-stdio-body-has-field body "\"source\":\"") 1)
        (push-object-vector-local with-uri (lsp-stdio-body-string-field body "\"source\":\""))
        (if (= (lsp-stdio-body-has-field body "\"text\":\"") 1)
          (push-object-vector-local with-uri (lsp-stdio-body-string-field body "\"text\":\""))
          with-uri))]
      (if (= (lsp-stdio-body-has-field body "\"path\":\"") 1)
        (push-object-vector-local with-source (lsp-stdio-body-string-field body "\"path\":\""))
        with-source))))

(defn lsp-stdio-rename-params [body]
  (let [params (vector-new 5)]
    (push-object-vector-local
      (push-object-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local params (lsp-stdio-body-int-field body "\"uri\":"))
            (lsp-stdio-body-int-field body "\"line\":"))
          (lsp-stdio-body-int-field-or body "\"col\":" "\"character\":"))
        (lsp-stdio-body-string-field body "\"source\":\""))
      (lsp-stdio-body-string-field body "\"newName\":\""))))

(defn lsp-stdio-diagnostic [body start]
  (let [severity (lsp-stdio-body-int-field-from body "\"severity\":" start)
    rule-id (lsp-stdio-body-int-field-from body "\"rule\":" start)
    line (lsp-stdio-body-int-field-from body "\"line\":" start)
    col (lsp-stdio-body-int-field-from body "\"col\":" start)
    message-hash (lsp-stdio-body-int-field-from body "\"messageHash\":" start)
    source (lsp-stdio-body-int-field-from body "\"source\":" start)
    diag (vector-new 6)]
    (push-int-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local
            (push-int-vector-local
              (push-int-vector-local diag severity)
              rule-id)
            line)
          col)
        message-hash)
      source)))

(defn lsp-stdio-diagnostics-loop [body idx len diagnostics]
  (let [source-pos (lsp-stdio-find-pattern-loop body "\"source\":" idx len)]
    (if (< source-pos 0)
      diagnostics
      (lsp-stdio-diagnostics-loop
        body
        (+ source-pos 1)
        len
        (push-object-vector-local diagnostics (lsp-stdio-diagnostic body source-pos))))))

(defn lsp-stdio-publish-diagnostics-params [body]
  (let [uri (lsp-stdio-body-int-field body "\"uri\":")
    diagnostics (lsp-stdio-diagnostics-loop body 0 (string-length body) (vector-new 4))]
    (push-object-vector-local (push-int-vector-local (vector-new 2) uri) diagnostics)))

(defn lsp-stdio-body-params [body]
  (let [method-id (lsp-stdio-body-method body)]
    (if (= method-id (lsp-method-goto-def))
      (lsp-stdio-nav-params body)
      (if (= method-id (lsp-method-hover))
        (lsp-stdio-nav-params body)
        (if (= method-id (lsp-method-references))
          (lsp-stdio-nav-params body)
          (if (= method-id (lsp-method-completion))
            (lsp-stdio-nav-params body)
            (if (= method-id (lsp-method-rename))
              (lsp-stdio-rename-params body)
              (if (= method-id (lsp-method-formatting))
                (lsp-stdio-document-params body)
                (if (= method-id (lsp-method-did-open))
                  (lsp-stdio-document-params body)
                  (if (= method-id (lsp-method-did-change))
                    (lsp-stdio-document-params body)
                    (if (= method-id (lsp-method-publish-diagnostics))
                      (lsp-stdio-publish-diagnostics-params body)
                      0)))))))))))

(defn lsp-stdio-body-message [body]
  (let [method-id (lsp-stdio-body-method body)
    msg (vector-new 4)]
    (push-object-vector-local
      (push-int-vector-local
        (push-int-vector-local
          (push-int-vector-local msg 2)
          (lsp-stdio-body-id body))
        method-id)
      (lsp-stdio-body-params body))))
(defn lsp-stdio-wire-loop [state wire idx len out] (if (>= idx len) out (let [header-end (lsp-stdio-find-header-end-loop wire idx len) header (substring wire idx header-end) content-length (lsp-parse-content-length header) body-start (+ header-end 4) body-end (+ body-start content-length) body (substring wire body-start body-end) rendered (lsp-transport-dispatch-request state (lsp-stdio-message-request (lsp-stdio-body-message body)))] (lsp-stdio-wire-loop state wire body-end len (string-concat out rendered)))))
(defn run-lsp-stdio-wire [wire] (let [state (server-state-new)] (lsp-stdio-wire-loop state wire 0 (string-length wire) "")))
(defn run-lsp-stdio-server [] (let [wire (read-stdin)] (do (print-string (run-lsp-stdio-wire wire)) (exit-success))))
(defn run-lsp [opts] (let [summary (lsp-init-summary) caps (lsp-init-capabilities summary)] (do (print-string (string-concat "sync:" (lsp-sync-kind-text (vector-get caps 0)))) (print-string "
") (print-string (string-concat "hover:" (lsp-bool-text (vector-get caps 1)))) (print-string "
") (print-string (string-concat "completion:" (lsp-bool-text (vector-get caps 2)))) (print-string "
") (print-string (string-concat "definition:" (lsp-bool-text (vector-get caps 3)))) (print-string "
") (print-string (string-concat "references:" (lsp-bool-text (vector-get caps 4)))) (print-string "
") (print-string (string-concat "rename:" (lsp-bool-text (vector-get caps 5)))) (print-string "
") (print-string (string-concat "formatting:" (lsp-bool-text (vector-get caps 6)))) (print-string "
") (print-string (lsp-summary-requests-text summary)) (print-string "
") (print-string (lsp-summary-documents-text summary)) (print-string "
") (print-string (lsp-summary-source-bytes-text summary)) (print-string "
") (exit-success))))
(defn run-fmt [file-path opts] (if (file-exists? file-path) (run-fmt-source (read-file file-path) opts) (exit-compile-error)))
(defn run-doc [file-path opts] (if (file-exists? file-path) (run-doc-source (read-file file-path) opts) (exit-compile-error)))
(defn parse-diagnostics-loop [spans pos-ref src diagnostics] (if (== (p-current spans pos-ref) 99) diagnostics (let [before (ref-get pos-ref) parsed (parse-with-recovery spans pos-ref src diagnostics) next-diagnostics (vector-get parsed 1)] (if (= (ref-get pos-ref) before) (do (p-advance pos-ref) (parse-diagnostics-loop spans pos-ref src next-diagnostics)) (parse-diagnostics-loop spans pos-ref src next-diagnostics)))))
(defn parse-diagnostics [src] (let [spans (tokenize-with-spans src) pos-ref (ref-new 0) diagnostics (parse-diagnostics-loop spans pos-ref src (collect-diagnostics))] diagnostics))
(defn parse-diagnostics-count [src] (let [diagnostics (parse-diagnostics src)] (vector-length diagnostics)))
(defn check-diagnostics-count-program [program] (infer-program-analysis-diagnostic-count (infer-program-analysis program)))
(defn check-diagnostics-first-code [program] (infer-program-analysis-first-error-code (infer-program-analysis program)))
(defn check-diagnostics-count [src] (let [program (parse-program src)] (check-diagnostics-count-program program)))
(defn dispatch-command-tail [cmd-id file-path opts] (if (= cmd-id (cmd-doc-ack)) (run-doc-ack file-path opts) (if (= cmd-id (cmd-doc-check)) (run-doc-check file-path opts) (if (= cmd-id (cmd-install)) (run-install file-path opts) (if (= cmd-id (cmd-repl)) (run-repl opts) (if (= cmd-id (cmd-lsp)) (run-lsp opts) (if (= cmd-id (cmd-fmt)) (run-fmt file-path opts) (if (= cmd-id (cmd-doc)) (run-doc file-path opts) (exit-unknown-command)))))))))
(defn dispatch-command [cmd-id file-path opts] (if (= cmd-id (cmd-parse)) (run-parse file-path opts) (if (= cmd-id (cmd-check)) (run-check file-path opts) (if (= cmd-id (cmd-compile)) (run-compile file-path opts) (if (= cmd-id (cmd-build)) (run-build file-path opts) (if (= cmd-id (cmd-test)) (run-test file-path opts) (if (= cmd-id (cmd-review)) (run-review file-path opts) (dispatch-command-tail cmd-id file-path opts))))))))
(defn help-text [] "Usage: lsharp <command> [options] Commands: parse check compile build test review doc-ack doc-check install repl lsp fmt doc")
(defn version-text [] "lsharp 0.1.0")
(defn show-help [] (do (print-string (help-text)) (exit-success)))
(defn show-version [] (do (print-string (version-text)) (exit-success)))
(defn cli-stdout [msg] (do (print-string msg) (print-string "
") 0))
(defn cli-stderr [msg] (do (print-string (string-concat "error: " msg)) (print-string "
") 0))
(defn format-subcommand-help [cmd] (if (string-eq cmd "parse") "parse <file> - Parse source and show AST" (if (string-eq cmd "check") "check <file> - Type-check source" (if (string-eq cmd "compile") "compile <file> [-o <file>] [--target <wasi-preview1|wasi-component|wasm>] - Compile to Wasm" (if (string-eq cmd "build") "build <file> [--output <file>] [--target <wasi-preview1|wasi-component|wasm>] - Build project" (if (string-eq cmd "test") "test <file> - Run metadata tests" (if (string-eq cmd "review") "review <file> - Code review" (if (string-eq cmd "doc-ack") "doc-ack <file> - Acknowledge docs" (if (string-eq cmd "doc-check") "doc-check <file> - Check doc consistency" (if (string-eq cmd "install") "install <pkg> - Install package" (if (string-eq cmd "repl") "repl - Interactive REPL" (if (string-eq cmd "lsp") "lsp [--stdio] - Start LSP server" (if (string-eq cmd "fmt") "fmt <file> - Format source" (if (string-eq cmd "doc") "doc <file> - Generate docs" "unknown command"))))))))))))))
(defn run-command [cmd-name file-path opts] (if (or (string-eq cmd-name "--help") (string-eq cmd-name "-h")) (show-help) (if (or (string-eq cmd-name "--version") (string-eq cmd-name "-v")) (show-version) (if (string-eq cmd-name "help") (do (cli-stdout (format-subcommand-help file-path)) (exit-success)) (if (or (string-eq file-path "--help") (string-eq file-path "-h")) (do (cli-stdout (format-subcommand-help cmd-name)) (exit-success)) (let [cmd-id (arg-parse cmd-name)] (if (= cmd-id 0) (do (cli-stderr (string-concat "unknown command: " cmd-name)) (exit-code-unknown-command)) (dispatch-command cmd-id file-path opts))))))))
(defn main-dispatch [cmd-name file-path opts] (run-command cmd-name file-path opts))
(defn compile-or-build-command [cmd-name] (or (string-eq cmd-name "compile") (string-eq cmd-name "build")))
(defn output-option-flag [arg] (or (string-eq arg "-o") (string-eq arg "--output")))
(defn target-option-flag [arg] (string-eq arg "--target"))
(defn json-option-flag [arg] (string-eq arg "--json"))
(defn format-option-flag [arg] (string-eq arg "--format"))
(defn cli-option-status-ok [] 0)
(defn cli-option-status-invalid-target [] 1)
(defn cli-option-status-missing-value [] 2)
(defn cli-option-status-unsupported-option [] 3)
(defn cli-option-result [status target output-path detail] (let [result (vector-new 4)] (push-object-vector-local (push-object-vector-local (push-int-vector-local (push-int-vector-local result status) target) output-path) detail)))
(defn cli-option-result-status [result] (vector-get result 0))
(defn cli-option-result-target [result] (vector-get result 1))
(defn cli-option-result-output-path [result] (vector-get result 2))
(defn cli-option-result-detail [result] (vector-get result 3))
(defn parse-cli-options-loop [idx argc target output-path] (if (>= idx argc) (cli-option-result (cli-option-status-ok) target output-path "") (let [flag (command-line-arg idx)] (if (>= (+ idx 1) argc) (cli-option-result (cli-option-status-missing-value) target output-path flag) (let [flag-value (command-line-arg (+ idx 1))] (if (output-option-flag flag) (parse-cli-options-loop (+ idx 2) argc target flag-value) (if (target-option-flag flag) (let [parsed-target (parse-compile-target-name flag-value)] (if (< parsed-target 0) (cli-option-result (cli-option-status-invalid-target) target output-path flag-value) (parse-cli-options-loop (+ idx 2) argc parsed-target output-path))) (cli-option-result (cli-option-status-unsupported-option) target output-path flag))))))))
(defn parse-cli-options [argc] (parse-cli-options-loop 2 argc (default-compile-target) ""))
(defn cli-option-error-message [result] (let [status (cli-option-result-status result) detail (cli-option-result-detail result)] (if (= status (cli-option-status-invalid-target)) (string-concat "unsupported target: " detail) (if (= status (cli-option-status-missing-value)) (string-concat "missing value for option: " detail) (string-concat "unsupported option: " detail)))))
(defn run-command-with-cli-options [cmd-name file-path result] (let [target (cli-option-result-target result) output-path (cli-option-result-output-path result)] (if (> (string-length output-path) 0) (if (string-eq cmd-name "compile") (run-compile-output file-path output-path target) (if (string-eq cmd-name "build") (run-build-output file-path output-path target) (run-command cmd-name file-path target))) (run-command cmd-name file-path target))))
(defn doc-cli-option-none [] 0)
(defn doc-cli-option-invalid [] (- 0 1))
(defn doc-cli-option-error-message [argc]
  (if (> argc 3)
    (string-concat "unsupported option: " (command-line-arg 3))
    (if (> argc 2)
      (string-concat "unsupported option: " (command-line-arg 2))
      "unsupported option")))
(defn check-cli-option-none [] 0)
(defn check-cli-option-invalid [] (- 0 1))
(defn parse-check-cli-option [argc]
  (if (<= argc 2)
    (check-cli-option-none)
    (let [arg2 (command-line-arg 2)]
      (if (and (= argc 3) (json-option-flag arg2))
        (check-option-json)
        (if (= argc 4)
          (if (format-option-flag arg2)
            (if (string-eq (command-line-arg 3) "json")
              (check-option-json)
              (check-cli-option-invalid))
            (check-cli-option-invalid))
          (check-cli-option-invalid))))))
(defn parse-doc-cli-option [argc cmd-name]
  (if (<= argc 2)
    (doc-cli-option-none)
    (let [arg2 (command-line-arg 2)]
      (if (string-eq cmd-name "doc-ack")
        (if (and (= argc 3) (string-eq arg2 "--trailer")) (doc-option-trailer-only) (doc-cli-option-invalid))
        (if (string-eq cmd-name "doc-check")
          (if (and (= argc 3) (string-eq arg2 "--strict")) (doc-option-strict-check) (doc-cli-option-invalid))
          (if (string-eq cmd-name "review")
            (if (and (= argc 3) (json-option-flag arg2))
              (review-option-json)
              (if (= argc 4)
                (if (format-option-flag arg2)
                  (if (string-eq (command-line-arg 3) "json")
                    (review-option-json)
                    (doc-cli-option-invalid))
                  (doc-cli-option-invalid))
                (doc-cli-option-invalid)))
            (if (string-eq cmd-name "doc")
              (if (and (= argc 3) (json-option-flag arg2))
                (doc-option-json)
                (if (= argc 4)
                  (if (format-option-flag arg2)
                    (if (string-eq (command-line-arg 3) "json")
                      (doc-option-json)
                      (doc-cli-option-invalid))
                    (doc-cli-option-invalid))
                  (doc-cli-option-invalid)))
              (doc-cli-option-none))))))))
(defn run-command-with-doc-option [cmd-name file-path doc-option] (let [cmd-id (arg-parse cmd-name)] (if (= cmd-id (cmd-doc-ack)) (run-doc-ack file-path doc-option) (if (= cmd-id (cmd-doc-check)) (run-doc-check file-path doc-option) (run-command cmd-name file-path doc-option)))))
(defn run-main-command [argc cmd-name file-path]
  (if (and (string-eq cmd-name "lsp") (string-eq file-path "--stdio"))
    (run-lsp-stdio-server)
    (if (and (compile-or-build-command cmd-name) (> argc 2))
      (let [options (parse-cli-options argc)]
        (if (= (cli-option-result-status options) (cli-option-status-ok))
          (run-command-with-cli-options cmd-name file-path options)
          (do
            (cli-stderr (cli-option-error-message options))
            (exit-code-compile-error))))
      (if (and (string-eq cmd-name "check") (> argc 2))
        (let [check-option (parse-check-cli-option argc)]
          (if (>= check-option 0)
            (run-command cmd-name file-path check-option)
            (do
              (cli-stderr (doc-cli-option-error-message argc))
              (exit-code-compile-error))))
        (if (> argc 2)
          (let [doc-option (parse-doc-cli-option argc cmd-name)]
            (if (= doc-option (doc-cli-option-invalid))
              (do
                (cli-stderr (doc-cli-option-error-message argc))
                (exit-code-compile-error))
              (run-command-with-doc-option cmd-name file-path doc-option)))
          (run-command cmd-name file-path (default-compile-target)))))))
(defn exit-main [code] (do (proc-exit code) 0))
(defn main []
  (let [argc (command-line-args)]
    (if (= argc 0)
      (exit-main (show-help))
      (let [cmd-name (command-line-arg 0)
        file-path (if (> argc 1) (command-line-arg 1) "")]
        (exit-main (run-main-command argc cmd-name file-path))))))
