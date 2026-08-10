(module App.EmbeddedCli)
(import App.CompilerMode)
(import Backend.Wasm.Compiler)
(import Backend.Wasm.CompilerBase)
(import Syntax.Lexer)
(import Syntax.Parser)
(import Syntax.AST)
(import Tools.Doc.DocJson)
(import Tools.Doc.DocTools)
(import Tools.Test.TestRunner)
(import Tools.Test.PropertyRunner)
(import Types.TypeInfer)
(import Types.TypeInferApply)
(import Types.TypeInferBlock)
(import Types.TypeInferPattern)
(import Types.TypeInferRecord)
(import Types.TypeInferCore)
(import Types.TypeScheme)
(import Types.TypeInferAssertions)
(import Types.MetadataMigration)
(import Tools.Validation.IntentSource)
(import Tools.Validation.Evidence)
(import Tools.Validation.Stale)
(import Tools.Validation.ReviewIdentity)

(defn push-int-vector-local [dst value] (do (root_push dst) (let [next-dst (vector-push dst value)] (do (root_pop) next-dst))))
(defn push-object-vector-local [dst value] (do (root_push dst) (root_push value) (let [next-dst (vector-push dst value)] (do (root_pop) (root_pop) next-dst))))
(defn exit-success [] 0)
(defn exit-compile-error [] 1)
(defn exit-runtime-error [] 2)
(defn cmd-parse [] 1)
(defn cmd-check [] 2)
(defn cmd-compile [] 3)
(defn cmd-build [] 4)
(defn cmd-test [] 5)
(defn cmd-review [] 6)
(defn cmd-doc-ack [] 7)
(defn cmd-doc-check [] 8)
(defn cmd-fmt [] 9)
(defn cmd-validate [] 10)
(defn compile-target-preview1 [] 0)
(defn compile-target-component [] 1)
(defn compile-target-invalid [] (- 0 1))
(defn default-compile-target [] (compile-target-component))
(defn parse-compile-target-name [target-name] (if (string-eq target-name "wasi-preview1") (compile-target-preview1) (if (or (string-eq target-name "wasi-component") (string-eq target-name "wasm")) (compile-target-component) (compile-target-invalid))))
(defn arg-parse [cmd-name]
  (if (string-eq cmd-name "parse")
    (cmd-parse)
    (if (string-eq cmd-name "check")
      (cmd-check)
      (if (string-eq cmd-name "compile")
        (cmd-compile)
        (if (string-eq cmd-name "build")
          (cmd-build)
          (if (string-eq cmd-name "test")
            (cmd-test)
            (if (string-eq cmd-name "review")
              (cmd-review)
              (if (string-eq cmd-name "doc-ack")
                (cmd-doc-ack)
                (if (string-eq cmd-name "doc-check")
                  (cmd-doc-check)
                  (if (string-eq cmd-name "fmt")
                    (cmd-fmt)
                    (if (string-eq cmd-name "validate")
                      (cmd-validate)
                      0)))))))))))
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
(defn check-diagnostic-body-from-code [code] (if (= code (canonical-assertion-type-error-code)) "assert predicate type error" (if (= code (canonical-assertion-non-bool-code)) "assert predicate must be Bool" (if (= code (canonical-assertion-empty-code)) "assert requires at least one predicate" (if (= code (canonical-assertion-vacuous-code)) "assert predicate is vacuous" (if (= code (error-code-undefined)) "undefined symbol" (if (= code (error-code-if-cond)) "if condition must be Bool" (if (= code (error-code-if-branch)) "if branches must have same type" (if (= code (error-code-arg-mismatch)) "function argument type mismatch" (if (= code (error-code-infinite)) "infinite type" (if (= code (error-code-recursive-alias)) "recursive type alias" "type error")))))))))))
(defn check-case-diagnostic-body-from-code [code] (if (= code (canonical-case-type-error-code)) "case expression type error" (if (= code (canonical-case-value-error-code)) "case actual and expected types must be Int or Bool" (if (= code (canonical-case-empty-code)) "case requires at least one expectation" "case type error"))))
(defn check-property-diagnostic-body-from-code [code] (if (= code (canonical-property-type-error-code)) "property predicate type error" (if (= code (canonical-property-non-bool-code)) "property predicate must be Bool" (if (= code (canonical-property-empty-code)) "property requires typed binders, a postcondition, and positive cases" (if (= code (canonical-assertion-vacuous-code)) "property predicate is vacuous" "property predicate type error")))))
(defn check-diagnostics-body-text [program] (let [code (check-diagnostics-first-code program)] (if (= code 0) "" (check-diagnostic-body-from-code code))))
(defn check-option-json [] 1)
(defn test-option-json [] 1)
(defn check-exit-code [diagnostics-count] (if (> diagnostics-count 0) (exit-compile-error) (exit-success)))
(defn check-json-diagnostics [count first-error-code body]
  (let [fields0 ""
    fields1 (legacy-json-append-field fields0 (legacy-json-int-field "count" count))
    fields2 (legacy-json-append-field fields1 (legacy-json-int-field "firstErrorCode" first-error-code))
    fields3 (legacy-json-append-field fields2 (legacy-json-field "message" body))]
    (string-concat "{" (string-concat fields3 "}"))))
(defn check-json-report [rendered diagnostics-count first-error-code diagnostics-body migration-rows failure-kinds]
  (let [fields0 ""
    fields1 (legacy-json-append-field fields0 (legacy-json-field "command" "check"))
    fields2 (legacy-json-append-field fields1 (legacy-json-field "type" rendered))
    diagnostics (check-json-diagnostics diagnostics-count first-error-code diagnostics-body)
    fields3 (legacy-json-append-field fields2 (string-concat "\"diagnostics\":" diagnostics))
    fields4 (legacy-json-append-field fields3 (string-concat "\"migration\":" (legacy-migration-detail-json-summary migration-rows)))
    fields5 (legacy-json-append-field fields4 (string-concat "\"failureKinds\":" (check-failure-kinds-json failure-kinds)))]
    (string-concat "{" (string-concat fields5 "}"))))
(defn parse-diagnostics-loop [spans pos-ref src diagnostics] (if (== (p-current spans pos-ref) 99) diagnostics (let [before (ref-get pos-ref) parsed (parse-with-recovery spans pos-ref src diagnostics) next-diagnostics (vector-get parsed 1)] (if (= (ref-get pos-ref) before) (do (p-advance pos-ref) (parse-diagnostics-loop spans pos-ref src next-diagnostics)) (parse-diagnostics-loop spans pos-ref src next-diagnostics)))))
(defn parse-diagnostics [src]
  (let [spans (tokenize-with-spans src)
    pos-ref (ref-new 0)
    delimiter-diagnostics (parse-delimiter-diagnostics spans src)]
    (if (> (vector-length delimiter-diagnostics) 0)
      delimiter-diagnostics
      (parse-diagnostics-loop spans pos-ref src (collect-diagnostics)))))
(defn check-diagnostics-count-program [program] (infer-program-analysis-diagnostic-count (infer-program-analysis program)))
(defn check-diagnostics-first-code [program] (infer-program-analysis-first-error-code (infer-program-analysis program)))
(defn builtin-type-name-text [type-hash] (if (= type-hash 100) "Int" (if (= type-hash 200) "Bool" (if (= type-hash 300) "String" (if (= type-hash 400) "Float" (if (= type-hash 500) "Unit" (string-concat "type-" (int-to-string type-hash))))))))
(defn render-type-text [ty] (let [tag (ty-tag ty)] (if (= tag 1) (builtin-type-name-text (ty-name ty)) (if (= tag 2) (string-concat "t" (int-to-string (ty-name ty))) (if (= tag 3) "Fn" (if (= tag 4) (string-concat "record-" (int-to-string (ty-name ty))) "Unknown"))))))
(defn check-failure-kinds-text-loop [kinds idx count]
  (if (>= idx count)
    ""
    (string-concat
      (if (= idx 0) "" ",")
      (string-concat
        (int-to-string (vector-get kinds idx))
        (check-failure-kinds-text-loop kinds (+ idx 1) count)))))
(defn check-failure-kinds-text [kinds]
  (string-concat "failure-kinds:" (check-failure-kinds-text-loop kinds 0 (vector-length kinds))))
(defn check-failure-kinds-json-loop [kinds idx count]
  (if (>= idx count)
    ""
    (string-concat
      (if (= idx 0) "" ",")
      (string-concat
        (int-to-string (vector-get kinds idx))
        (check-failure-kinds-json-loop kinds (+ idx 1) count)))))
(defn check-failure-kinds-json [kinds]
  (string-concat "[" (string-concat (check-failure-kinds-json-loop kinds 0 (vector-length kinds)) "]")))
(defn run-parse-source [src opts] (let [program (parse-program src) diagnostics (parse-diagnostics src) diagnostics-count (vector-length diagnostics) diagnostics-text (diagnostics-summary-text diagnostics-count "P0001" (parse-diagnostics-body-text diagnostics))] (do (print-string (parse-decl-count-text program)) (print-string "\n") (print-string (string-concat "first-decl:" (parse-first-decl-text program))) (print-string "\n") (print-string (string-concat "first-body:" (parse-first-body-text program))) (print-string "\n") (print-string diagnostics-text) (print-string "\n") (exit-success))))
(defn run-check-program [context opts]
  (let [context-root-slot (root_push context)
    program (vector-get context 0)
    module-owners (vector-get context 1)
    program-root-slot (root_push program)
    analysis (infer-program-analysis program)
    analysis-root-slot (root_push analysis)
    ty (infer-program-analysis-type analysis)
    rendered (render-type-text ty)
    base-diagnostics-count (infer-program-analysis-diagnostic-count analysis)
    base-first-error-code (infer-program-analysis-first-error-code analysis)
    base-first-error-index (infer-program-analysis-first-error-index analysis)
    base-first-error-start (infer-program-analysis-first-error-start analysis)
    base-first-error-end (infer-program-analysis-first-error-end analysis)
    base-failure-kinds (infer-program-analysis-failure-kinds analysis)
    first-module-owner
      (if (and (> base-diagnostics-count 0)
        (and (>= base-first-error-index 0)
          (< base-first-error-index (vector-length module-owners))))
        (vector-get module-owners base-first-error-index)
        (vector-new 0))
    first-module-hash (if (> (vector-length first-module-owner) 0) (vector-get first-module-owner 0) -1)
    first-module-name (if (> (vector-length first-module-owner) 1) (vector-get first-module-owner 1) "")
    first-module-path (if (> (vector-length first-module-owner) 2) (vector-get first-module-owner 2) "")
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
        (print-string (check-json-report rendered diagnostics-count first-error-code diagnostics-body migration-rows base-failure-kinds))
        (print-string "\n")
        (let [exit-code (check-exit-code diagnostics-count)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            exit-code)))
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
      (if (>= first-module-hash 0)
        (do
          (print-string (string-concat "first-module-hash:" (int-to-string first-module-hash)))
          (print-string "\n")
          (if (> (string-length first-module-name) 0)
            (do
              (print-string (string-concat "first-module-name:" first-module-name))
              (print-string "\n"))
            (print-string ""))
          (if (> (string-length first-module-path) 0)
            (do
              (print-string (string-concat "first-module-path:" first-module-path))
              (print-string "\n"))
            (print-string ""))
          (if (and (>= base-first-error-start 0) (>= base-first-error-end base-first-error-start))
            (do
              (print-string
                (string-concat
                  "first-error-span:"
                  (string-concat
                    (int-to-string base-first-error-start)
                    (string-concat ":" (int-to-string base-first-error-end)))))
              (print-string "\n"))
            (print-string "")))
        (print-string ""))
        (if (> base-diagnostics-count 0)
          (do
            (print-string (check-failure-kinds-text base-failure-kinds))
            (print-string "\n"))
          (print-string ""))
        (let [exit-code (check-exit-code diagnostics-count)]
          (do
            (root_pop)
            (root_pop)
            (root_pop)
            exit-code))))))
;; EC-M2-01 の最初の selfhost validation slice。parser が保持した
;; defn metadata を source node/edge の wire payload として集計し、未接続
;; の trace gap を unknown report へ投影する。evidence registry は後段で接続する。
(defn validation-state-new []
  (let [state0 (vector-new 5)
    state1 (push-object-vector-local state0 (ref-new (vector-new 0)))
    state2 (push-object-vector-local state1 (ref-new (vector-new 0)))
    state3 (push-object-vector-local state2 (ref-new (vector-new 0)))
    state4 (push-object-vector-local state3 (ref-new (vector-new 0)))]
    (push-object-vector-local state4 (ref-new 0))))
(defn validation-state-add-object [state slot value]
  (let [items-ref (vector-get state slot)
    items (ref-get items-ref)
    updated (push-object-vector-local items value)]
    (do
      (ref-set items-ref updated)
      state)))
(defn validation-source-pair [left right]
  (let [pair (vector-new 2)
    with-left (push-object-vector-local pair left)]
    (push-object-vector-local with-left right)))
(defn validation-defn-metadata [decl]
  (let [decl-len (vector-length decl)
    body-end (+ 4 (vector-get decl 2))]
    (if (< body-end decl-len)
      (vector-get decl body-end)
      (vector-new 0))))
(defn validation-defn-forms [decl]
  (let [meta (validation-defn-metadata decl)]
    (if (> (vector-length meta) 5)
      (vector-get meta 5)
      (vector-new 0))))
(defn validation-state-consume-form [state form]
  (let [kind (vector-get form 0)
    payload (vector-get form 1)
    payload-len (vector-length payload)]
    (if (= kind 6)
      (if (= payload-len 2)
        (validation-state-add-object state 0 (vector-get payload 0))
        state)
      (if (= kind 7)
        (if (= payload-len 2)
          (validation-state-add-object state 1 (vector-get payload 0))
          state)
        (if (= kind 9)
          (do
            (ref-set (vector-get state 4) (+ (ref-get (vector-get state 4)) 1))
            state)
          (if (= kind 10)
            (if (= payload-len 2)
              (validation-state-add-object
                state
                2
                (validation-source-pair (vector-get payload 0) (vector-get payload 1)))
              state)
            (if (= kind 12)
              (if (= payload-len 2)
                (validation-state-add-object
                  state
                  3
                  (validation-source-pair (vector-get payload 0) (vector-get payload 1)))
                state)
              state)))))))
(defn validation-forms-loop [forms idx state]
  (if (>= idx (vector-length forms))
    state
    (validation-forms-loop
      forms
      (+ idx 1)
      (validation-state-consume-form state (vector-get forms idx)))))
(defn validation-program-loop [program idx state]
  (if (>= idx (vector-length program))
    state
    (let [decl (vector-get program idx)
      next-state
        (if (= (vector-get decl 0) (ast-defn))
          (validation-forms-loop (validation-defn-forms decl) 0 state)
          state)]
      (validation-program-loop program (+ idx 1) next-state))))
(defn validation-source-state [program]
  (validation-program-loop program 0 (validation-state-new)))
(defn validation-edge-links-id? [edges idx target]
  (if (>= idx (vector-length edges))
    0
    (let [edge (vector-get edges idx)]
      (if (string-eq (vector-get edge 0) target)
        1
        (validation-edge-links-id? edges (+ idx 1) target)))))
(defn validation-gap-json [code subject]
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-string-field "code" code))
    fields2 (docjson-append fields1 (docjson-string-field "subject_id" subject))]
    (docjson-object-wrap fields2)))
(defn validation-intent-gaps-loop [intents motives idx out]
  (if (>= idx (vector-length intents))
    out
    (let [intent (vector-get intents idx)
      next-out
        (if (= (validation-edge-links-id? motives 0 intent) 1)
          out
          (docjson-append
            out
            (validation-gap-json "trace-gap.intent-without-claim" intent)))]
      (validation-intent-gaps-loop intents motives (+ idx 1) next-out))))
(defn validation-claim-gaps-loop [claims tested-by idx out]
  (if (>= idx (vector-length claims))
    out
    (let [claim (vector-get claims idx)
      next-out
        (if (= (validation-edge-links-id? tested-by 0 claim) 1)
          out
          (docjson-append
            out
            (validation-gap-json "trace-gap.claim-without-test" claim)))]
      (validation-claim-gaps-loop claims tested-by (+ idx 1) next-out))))
(defn validation-intent-gap-count-loop [intents motives idx count]
  (if (>= idx (vector-length intents))
    count
    (validation-intent-gap-count-loop
      intents
      motives
      (+ idx 1)
      (if (= (validation-edge-links-id? motives 0 (vector-get intents idx)) 1)
        count
        (+ count 1)))))
(defn validation-claim-gap-count-loop [claims tested-by idx count]
  (if (>= idx (vector-length claims))
    count
    (validation-claim-gap-count-loop
      claims
      tested-by
      (+ idx 1)
      (if (= (validation-edge-links-id? tested-by 0 (vector-get claims idx)) 1)
        count
        (+ count 1)))))
(defn validation-trace-gap-count [state]
  (let [intents (ref-get (vector-get state 0))
    claims (ref-get (vector-get state 1))
    motives (ref-get (vector-get state 2))
    tested-by (ref-get (vector-get state 3))
    intent-count (validation-intent-gap-count-loop intents motives 0 0)]
    (validation-claim-gap-count-loop claims tested-by 0 intent-count)))
(defn validation-trace-gaps-json [state]
  (let [intents (ref-get (vector-get state 0))
    claims (ref-get (vector-get state 1))
    motives (ref-get (vector-get state 2))
    tested-by (ref-get (vector-get state 3))
    intent-gaps (validation-intent-gaps-loop intents motives 0 "")
    all-gaps (validation-claim-gaps-loop claims tested-by 0 intent-gaps)]
    (docjson-array-wrap all-gaps)))
(defn validation-string-id-exists-loop [ids id idx len]
  (if (>= idx len)
    0
    (if (string-eq (vector-get ids idx) id)
      1
      (validation-string-id-exists-loop ids id (+ idx 1) len))))
(defn validation-string-id-exists? [ids id]
  (validation-string-id-exists-loop ids id 0 (vector-length ids)))
(defn validation-add-evidence-id [ids id]
  (if (= (validation-string-id-exists? ids id) 1)
    ids
    (push-object-vector-local ids id)))
(defn validation-independent-review-count-loop [registry idx len count]
  (if (>= idx len)
    count
    (let [evidence-record (vector-get registry idx)
      next-count
        (if (and
              (string-eq (source-evidence-record-method evidence-record) "review")
              (string-eq (source-evidence-record-independence evidence-record) "independent-review"))
          (+ count 1)
          count)]
      (validation-independent-review-count-loop registry (+ idx 1) len next-count))))
(defn validation-contradictory-records-loop [registry idx len ids]
  (if (>= idx len)
    ids
    (let [evidence-record (vector-get registry idx)
      next-ids
        (if (string-eq (source-evidence-record-outcome evidence-record) "contradicted")
          (validation-add-evidence-id ids (source-evidence-record-id evidence-record))
          ids)]
      (validation-contradictory-records-loop registry (+ idx 1) len next-ids))))
(defn validation-contradictory-edges-loop [edges idx len ids]
  (if (>= idx len)
    ids
    (let [edge (vector-get edges idx)
      next-ids
        (if (= (source-edge-kind edge) (source-edge-contradicts))
          (validation-add-evidence-id ids (source-edge-left edge))
          ids)]
      (validation-contradictory-edges-loop edges (+ idx 1) len next-ids))))
(defn validation-evidence-metrics [graph]
  (let [registry (source-evidence-graph-registry graph)
    edges (source-graph-edges graph)
    independent-reviews
      (validation-independent-review-count-loop registry 0 (vector-length registry) 0)
    ids0 (vector-new 0)
    ids1 (validation-contradictory-records-loop registry 0 (vector-length registry) ids0)
    ids2 (validation-contradictory-edges-loop edges 0 (vector-length edges) ids1)
    stale-metrics (source-evidence-stale-metrics graph)
    metrics0 (vector-new 0)
    metrics1 (push-int-vector-local metrics0 independent-reviews)
    metrics2 (push-int-vector-local metrics1 (vector-length ids2))
    metrics3 (push-int-vector-local metrics2 (vector-get stale-metrics 0))]
    (push-int-vector-local metrics3 (vector-get stale-metrics 1))))
(defn validation-source-review-id-order-loop [left right idx limit]
  (if (>= idx limit)
    (if (> (string-length left) (string-length right)) 1
      (if (< (string-length left) (string-length right)) (- 0 1) 0))
    (let [left-char (string-char-at left idx)
      right-char (string-char-at right idx)]
      (if (= left-char right-char)
        (validation-source-review-id-order-loop left right (+ idx 1) limit)
        (if (> left-char right-char) 1 (- 0 1))))))
(defn validation-source-review-id-order [left right]
  (let [left-len (string-length left)
    right-len (string-length right)
    limit (if (< left-len right-len) left-len right-len)]
    (validation-source-review-id-order-loop left right 0 limit)))
(defn validation-source-review-attestation-after? [left right]
  (= (validation-source-review-id-order
       (source-review-attestation-id left)
       (source-review-attestation-id right)) 1))
(defn validation-source-review-attestation-copy [src from to out]
  (if (>= from to)
    out
    (validation-source-review-attestation-copy
      src
      (+ from 1)
      to
      (push-object-vector-local out (vector-get src from)))))
(defn validation-source-review-attestation-insert [sorted elem idx]
  (if (= idx 0)
    (let [out (vector-new (+ (vector-length sorted) 1))
      out (push-object-vector-local out elem)]
      (validation-source-review-attestation-copy sorted 0 (vector-length sorted) out))
    (let [prev (vector-get sorted (- idx 1))]
      (if (validation-source-review-attestation-after? prev elem)
        (validation-source-review-attestation-insert sorted elem (- idx 1))
        (let [out (vector-new (+ (vector-length sorted) 1))
          out (validation-source-review-attestation-copy sorted 0 idx out)
          out (push-object-vector-local out elem)]
          (validation-source-review-attestation-copy sorted idx (vector-length sorted) out))))))
(defn validation-source-review-attestation-sort-loop [attestations sorted idx len]
  (if (>= idx len)
    sorted
    (let [elem (vector-get attestations idx)
      next-sorted
        (validation-source-review-attestation-insert
          sorted
          elem
          (vector-length sorted))]
      (validation-source-review-attestation-sort-loop
        attestations
        next-sorted
        (+ idx 1)
        len))))
(defn validation-source-review-attestation-sort [attestations]
  (let [len (vector-length attestations)]
    (if (< len 2)
      attestations
      (let [first (vector-get attestations 0)
        initial (push-object-vector-local (vector-new 1) first)]
        (validation-source-review-attestation-sort-loop attestations initial 1 len)))))
(defn validation-source-review-verification-json [attestation]
  (let [fields0
      (docjson-string-field "review_id" (source-review-attestation-id attestation))
    fields1
      (docjson-append
        fields0
        (docjson-string-field "state" (source-review-attestation-state attestation)))]
    (docjson-object-wrap fields1)))
(defn validation-source-review-verifications-json-loop [attestations idx len out]
  (if (>= idx len)
    out
    (validation-source-review-verifications-json-loop
      attestations
      (+ idx 1)
      len
      (docjson-append
        out
        (validation-source-review-verification-json (vector-get attestations idx))))))
(defn validation-source-review-verifications-json [attestations]
  (docjson-array-wrap
    (validation-source-review-verifications-json-loop
      attestations
      0
      (vector-length attestations)
      "")))
(defn validation-source-status-code [state independent-reviews contradicting-observations stale-reviews stale-evidence]
  (if (> contradicting-observations 0)
    1
    (if (or (> stale-reviews 0) (> stale-evidence 0))
      2
      (if (> (validation-trace-gap-count state) 0)
      2
      (if (> (ref-get (vector-get state 4)) 0)
        2
        (if (= independent-reviews 0) 2 0))))))
(defn validation-source-report-json
  [state independent-reviews contradicting-observations stale-reviews stale-evidence review-verifications review-evidence-identity]
  (let [fields0 ""
    status-code (validation-source-status-code
      state
      independent-reviews
      contradicting-observations
      stale-reviews
      stale-evidence)
    status (if (= status-code 1) "fail" (if (= status-code 0) "pass" "unknown"))
    fields1 (docjson-append fields0 (docjson-string-field "status" status))
    fields2 (docjson-append fields1 (docjson-array-field "trace_gaps" (validation-trace-gaps-json state)))
    fields3 (docjson-append fields2 (docjson-int-field "open_questions" (ref-get (vector-get state 4))))
    fields4 (docjson-append fields3 (docjson-int-field "independent_reviews" independent-reviews))
    fields5 (docjson-append fields4 (docjson-int-field "contradicting_observations" contradicting-observations))
    fields6 (docjson-append fields5 (docjson-int-field "stale_reviews" stale-reviews))
    fields7 (docjson-append fields6 (docjson-int-field "stale_evidence" stale-evidence))
    fields8
      (if (> (vector-length review-evidence-identity) 0)
        (docjson-append
          fields7
          (docjson-object-field
            "review_evidence_identity"
            (source-review-evidence-identity-json review-evidence-identity)))
        fields7)
    fields9
      (if (> (vector-length review-verifications) 0)
        (docjson-append
          fields8
          (docjson-array-field
            "review_verifications"
            (validation-source-review-verifications-json review-verifications)))
        fields8)
    fields10
      (if (> (vector-length review-verifications) 0)
        (docjson-append
          fields9
          (docjson-array-field
            "review_attestations"
            (validation-source-review-attestation-projections-json review-verifications)))
        fields9)]
    (docjson-object-wrap fields10)))
(defn validation-report-text-line [key value]
  (string-concat key (string-concat ": " value)))
(defn validation-report-text-append [out line]
  (if (= (string-length out) 0)
    line
    (string-concat out (string-concat "\n" line))))
(defn validation-gap-text [code subject]
  (validation-report-text-line code subject))
(defn validation-intent-gaps-text-loop [intents motives idx out]
  (if (>= idx (vector-length intents))
    out
    (let [intent (vector-get intents idx)
      next-out
        (if (= (validation-edge-links-id? motives 0 intent) 1)
          out
          (validation-report-text-append
            out
            (validation-gap-text "trace-gap.intent-without-claim" intent)))]
      (validation-intent-gaps-text-loop intents motives (+ idx 1) next-out))))
(defn validation-claim-gaps-text-loop [claims tested-by idx out]
  (if (>= idx (vector-length claims))
    out
    (let [claim (vector-get claims idx)
      next-out
        (if (= (validation-edge-links-id? tested-by 0 claim) 1)
          out
          (validation-report-text-append
            out
            (validation-gap-text "trace-gap.claim-without-test" claim)))]
      (validation-claim-gaps-text-loop claims tested-by (+ idx 1) next-out))))
(defn validation-trace-gaps-text [state]
  (let [intents (ref-get (vector-get state 0))
    claims (ref-get (vector-get state 1))
    motives (ref-get (vector-get state 2))
    tested-by (ref-get (vector-get state 3))
    intent-gaps (validation-intent-gaps-text-loop intents motives 0 "")]
    (validation-claim-gaps-text-loop claims tested-by 0 intent-gaps)))
(defn validation-source-review-verifications-text-loop [attestations idx len out]
  (if (>= idx len)
    out
    (let [attestation (vector-get attestations idx)
      line (validation-report-text-line
        "review-verification"
        (string-concat
          (source-review-attestation-id attestation)
          (string-concat "=" (source-review-attestation-state attestation))))]
      (validation-source-review-verifications-text-loop
        attestations
        (+ idx 1)
        len
        (validation-report-text-append out line)))))
(defn validation-source-review-identity-display [value]
  (if (> (string-length value) 0) value "-"))
(defn validation-source-review-identity-text [identity]
  (let [subject (source-review-evidence-identity-subject-digest identity)
    source (source-review-evidence-identity-source-commit identity)
    artifact (source-review-evidence-identity-artifact-digest identity)
    trust-store (validation-source-review-identity-display
      (source-review-evidence-identity-trust-store-digest identity))
    lifecycle (validation-source-review-identity-display
      (source-review-evidence-identity-lifecycle-digest identity))
    now (source-review-evidence-identity-now identity)]
    (string-concat
      "subject="
      (string-concat
        subject
        (string-concat
          " source="
          (string-concat
            source
            (string-concat
              " artifact="
              (string-concat
                artifact
                (string-concat
                  " trust-store="
                  (string-concat
                    trust-store
                    (string-concat
                      " lifecycle="
                      (string-concat lifecycle (string-concat " now=" now)))))))))))))
(defn validation-source-report-text
  [state independent-reviews contradicting-observations stale-reviews stale-evidence review-verifications review-evidence-identity]
  (let [status-code (validation-source-status-code
      state
      independent-reviews
      contradicting-observations
      stale-reviews
      stale-evidence)
    status (if (= status-code 1) "fail" (if (= status-code 0) "pass" "unknown"))
    trace-gaps (validation-trace-gaps-text state)
    line0 (validation-report-text-line "status" status)
    with-gaps
      (if (> (string-length trace-gaps) 0)
        (validation-report-text-append line0 trace-gaps)
        line0)
    line1 (validation-report-text-append
      with-gaps
      (validation-report-text-line
        "open-questions"
        (int-to-string (ref-get (vector-get state 4)))))
    line2 (validation-report-text-append
      line1
      (validation-report-text-line "independent-reviews" (int-to-string independent-reviews)))
    line3 (validation-report-text-append
      line2
      (validation-report-text-line
        "contradicting-observations"
        (int-to-string contradicting-observations)))
    line4 (validation-report-text-append
      line3
      (validation-report-text-line "stale-reviews" (int-to-string stale-reviews)))
    line5 (validation-report-text-append
      line4
      (validation-report-text-line "stale-evidence" (int-to-string stale-evidence)))
    with-verifications (validation-source-review-verifications-text-loop
      review-verifications
      0
      (vector-length review-verifications)
      line5)]
    (if (> (vector-length review-evidence-identity) 0)
      (validation-report-text-append
        with-verifications
        (validation-report-text-line
          "review-evidence-identity"
          (validation-source-review-identity-text review-evidence-identity)))
      with-verifications)))
(defn validation-option-manifest-path [opts] (vector-get opts 1))
(defn validate-options-review-subject-digest [result] (vector-get result 4))
(defn validate-options-review-source-commit [result] (vector-get result 5))
(defn validate-options-review-artifact-digest [result] (vector-get result 6))
(defn validate-options-review-trust-store-digest [result] (vector-get result 7))
(defn validate-options-review-lifecycle-digest [result] (vector-get result 8))
(defn validate-options-review-now [result] (vector-get result 9))
(defn validation-source-review-identity-result [opts]
  (let [subject (validate-options-review-subject-digest opts)]
    (if (= (string-length subject) 0)
      (source-result 1 (vector-new 0))
      (source-review-evidence-identity-result
        subject
        (validate-options-review-source-commit opts)
        (validate-options-review-artifact-digest opts)
        (validate-options-review-trust-store-digest opts)
        (validate-options-review-lifecycle-digest opts)
        (validate-options-review-now opts)))))
(defn validation-source-write-manifest [graph manifest-path]
  (if (> (string-length manifest-path) 0)
    (write-file manifest-path (validation-source-manifest-json graph))
    0))
(defn run-validate-source [src opts]
  (let [program (parse-program src)
    graph-result (source-evidence-graph-from-program program)]
    (if (= (source-result-status graph-result) 0)
      (let [error (source-result-error graph-result)]
        (do
          (cli-stderr
            (string-concat
              "source validation error:"
              (int-to-string (source-graph-error-code error))))
          (exit-compile-error)))
      (let [graph (source-result-value graph-result)
        identity-result (validation-source-review-identity-result opts)]
        (if (= (source-result-status identity-result) 0)
          (let [error (source-result-error identity-result)]
            (do
              (cli-stderr
                (string-concat
                  "source validation error:"
                  (int-to-string (source-graph-error-code error))))
              (exit-compile-error)))
          (let [identity (source-result-value identity-result)
            attached-result
              (if (> (vector-length identity) 0)
                (source-evidence-graph-attach-review-identity graph identity)
                (source-result 1 graph))]
            (if (= (source-result-status attached-result) 0)
              (let [error (source-result-error attached-result)]
                (do
                  (cli-stderr
                    (string-concat
                      "source validation error:"
                      (int-to-string (source-graph-error-code error))))
                  (exit-compile-error)))
              (let [graph2 (source-result-value attached-result)
                manifest-path (validation-option-manifest-path opts)
                state (validation-source-state program)
                metrics (validation-evidence-metrics graph2)
                independent-reviews (vector-get metrics 0)
                contradicting-observations (vector-get metrics 1)
                stale-reviews (vector-get metrics 2)
                stale-evidence (vector-get metrics 3)
                review-verifications
                  (validation-source-review-attestation-sort
                    (source-evidence-graph-attestations graph2))
                review-evidence-identity
                  (if (> (vector-length graph2) 5)
                    (source-evidence-graph-review-identity graph2)
                    (vector-new 0))
                status-code (validation-source-status-code
                  state
                  independent-reviews
                  contradicting-observations
                  stale-reviews
                  stale-evidence)
                report
                  (if (= (validate-options-status opts) (validate-option-text))
                    (validation-source-report-text
                      state
                      independent-reviews
                      contradicting-observations
                      stale-reviews
                      stale-evidence
                      review-verifications
                      review-evidence-identity)
                    (validation-source-report-json
                      state
                      independent-reviews
                      contradicting-observations
                      stale-reviews
                      stale-evidence
                      review-verifications
                      review-evidence-identity))]
                (if (and (> (string-length manifest-path) 0)
                    (< (validation-source-write-manifest graph2 manifest-path) 0))
                  (do
                    (cli-stderr "source validation manifest write failed")
                    (exit-compile-error))
                  (do
                    (print-string report)
                    (print-string "\n")
                    (if (= status-code 1)
                      (exit-compile-error)
                      (if (= status-code 0)
                        (exit-success)
                        (exit-runtime-error)))))))))))))
(defn run-check-source [src opts] (run-check-program (make-check-program-context (parse-program src) (vector-new 0)) opts))
(defn test-examples-text [count] (string-concat "examples:" (int-to-string count)))
(defn test-invariants-text [count] (string-concat "invariants:" (int-to-string count)))
(defn test-assertions-text [count] (string-concat "assertions:" (int-to-string count)))
(defn test-cases-text [count] (string-concat "cases:" (int-to-string count)))
(defn test-properties-text [count] (string-concat "properties:" (int-to-string count)))
(defn test-failures-text [count] (string-concat "failures:" (int-to-string count)))
(defn assurance-result-actual-loop [results idx count acc]
  (if (>= idx count)
    acc
    (assurance-result-actual-loop
      results
      (+ idx 1)
      count
      (+ acc (vector-get (vector-get results idx) 2)))))
(defn assurance-result-actual [results]
  (assurance-result-actual-loop results 0 (vector-length results) 0))
(defn assurance-total-actual [examples invariants assertions cases properties]
  (+
    (assurance-result-actual examples)
    (+
      (assurance-result-actual invariants)
      (+
        (assurance-result-actual assertions)
        (+ (assurance-result-actual cases) (assurance-result-actual properties))))))
(defn assurance-method [property-count case-count assertion-count example-count invariant-count]
  (if (> property-count 0)
    "sampled-property"
    (if (> case-count 0)
      "explicit-case"
      (if (> assertion-count 0)
        "assert"
        (if (or (> example-count 0) (> invariant-count 0))
          "legacy-deterministic-smoke"
          "none")))))
(defn assurance-generator [method]
  (if (string-eq method "sampled-property")
    "legacy-deterministic-smoke"
    "direct-evaluation"))
(defn assurance-status [failed diagnostic-count]
  (if (or (> failed 0) (> diagnostic-count 0)) "fail" "pass"))
(defn assurance-coverage-json [executed failed]
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-int-field "executed" executed))
    fields2 (docjson-append fields1 (docjson-int-field "failed" failed))]
    (docjson-object-wrap fields2)))
(defn assurance-diagnostic-span-json [start end]
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-int-field "start" start))
    fields2 (docjson-append fields1 (docjson-int-field "end" end))]
    (docjson-object-wrap fields2)))
(defn assurance-diagnostics-json
  [count first-error-code first-error-start first-error-end message]
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-int-field "count" count))
    fields2 (docjson-append fields1 (docjson-int-field "firstErrorCode" first-error-code))
    fields3 (docjson-append fields2
      (docjson-object-field
        "firstErrorSpan"
        (assurance-diagnostic-span-json first-error-start first-error-end)))
    fields4 (docjson-append fields3 (docjson-string-field "message" message))]
    (docjson-object-wrap fields4)))
(defn assurance-provenance-json []
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-string-field "runner" "selfhost"))
    fields2 (docjson-append fields1 (docjson-string-field "source_commit" "unknown"))
    fields3 (docjson-append fields2 (docjson-string-field "artifact_digest" "unknown"))]
    (docjson-object-wrap fields3)))
(defn assurance-intent-json []
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-string-field "status" "unknown"))
    fields2 (docjson-append fields1 (docjson-int-field "open_questions" 0))
    fields3 (docjson-append fields2 (docjson-int-field "independent_reviews" 0))
    fields4 (docjson-append fields3 (docjson-int-field "contradicting_observations" 0))]
    (docjson-object-wrap fields4)))
(defn assurance-conformance-json
  [status method cases executed failed diagnostic-count diagnostic-code diagnostic-start diagnostic-end diagnostic-message]
  (let [fields0 ""
    fields1 (docjson-append fields0 (docjson-string-field "status" status))
    fields2 (docjson-append fields1 (docjson-string-field "method" method))
    fields3 (docjson-append fields2 (docjson-int-field "cases" cases))
    fields4 (docjson-append fields3 (docjson-int-field "seed" 0))
    fields5 (docjson-append fields4 (docjson-string-field "generator" (assurance-generator method)))
    fields6 (docjson-append fields5 (docjson-array-field "shrinks" "[]"))
    fields7 (docjson-append fields6 (docjson-object-field "coverage" (assurance-coverage-json executed failed)))
    fields8 (docjson-append fields7
      (docjson-object-field
        "diagnostics"
        (assurance-diagnostics-json
          diagnostic-count
          diagnostic-code
          diagnostic-start
          diagnostic-end
          diagnostic-message)))
    fields9 (docjson-append fields8 (docjson-string-field "target" "unknown"))
    fields10 (docjson-append fields9 (docjson-object-field "provenance" (assurance-provenance-json)))]
    (docjson-object-wrap fields10)))
(defn assurance-report-json
  [status method cases executed failed diagnostic-count diagnostic-code diagnostic-start diagnostic-end diagnostic-message]
  (let [fields0 ""
    fields1 (docjson-append fields0
      (docjson-object-field
        "implementation_conformance"
        (assurance-conformance-json
          status
          method
          cases
          executed
          failed
          diagnostic-count
          diagnostic-code
          diagnostic-start
          diagnostic-end
          diagnostic-message)))
    fields2 (docjson-append fields1
      (docjson-object-field "intent_validation" (assurance-intent-json)))]
    (docjson-object-wrap fields2)))
(defn run-test-source-json-preflight [program diagnostic-code diagnostic-start diagnostic-end]
  (let [examples (extract-examples-from-program program)
    invariants (extract-invariants-from-program program)
    assertions (extract-assertions-from-program program)
    cases (extract-cases-from-program program)
    properties (extract-property-test-cases program)
    method (assurance-method
      (vector-length properties)
      (vector-length cases)
      (vector-length assertions)
      (vector-length examples)
      (vector-length invariants))
    rendered (assurance-report-json
      "fail"
      method
      0
      0
      1
      1
      diagnostic-code
      diagnostic-start
      diagnostic-end
      "")]
    (do
      (print-string rendered)
      (print-string "\n")
      (exit-runtime-error))))
(defn assurance-suite-failed [suite]
  (let [examples (vector-get suite 0)
    invariants (vector-get suite 1)
    assertions (vector-get suite 2)
    cases (vector-get suite 3)
    properties (vector-get suite 4)]
    (+
      (count-failed-results examples)
      (+
        (count-failed-results invariants)
        (+
          (count-failed-results assertions)
          (+ (count-failed-results cases) (count-failed-results properties)))))))
(defn assurance-suite-diagnostic-count [suite]
  (test-diagnostics-count-with-properties
    (vector-get suite 0)
    (vector-get suite 1)
    (vector-get suite 2)
    (vector-get suite 3)
    (vector-get suite 4)))
(defn assurance-suite-diagnostic-code [suite]
  (first-test-diagnostic-code-with-properties
    (vector-get suite 0)
    (vector-get suite 1)
    (vector-get suite 2)
    (vector-get suite 3)
    (vector-get suite 4)))

(defn assurance-suite-diagnostic-message [suite]
  (first-test-diagnostic-message-with-properties
    (vector-get suite 0)
    (vector-get suite 1)
    (vector-get suite 2)
    (vector-get suite 3)
    (vector-get suite 4)))

(defn assurance-suite-diagnostic-span [suite]
  (first-test-diagnostic-span-with-properties
    (vector-get suite 0)
    (vector-get suite 1)
    (vector-get suite 2)
    (vector-get suite 3)
    (vector-get suite 4)))
(defn assurance-suite-method [suite]
  (assurance-method
    (vector-length (vector-get suite 4))
    (vector-length (vector-get suite 3))
    (vector-length (vector-get suite 2))
    (vector-length (vector-get suite 0))
    (vector-length (vector-get suite 1))))
(defn assurance-suite-executed [suite]
  (assurance-total-actual
    (vector-get suite 0)
    (vector-get suite 1)
    (vector-get suite 2)
    (vector-get suite 3)
    (vector-get suite 4)))
(defn run-test-source-json-suite [suite]
  (let [failed (assurance-suite-failed suite)
    diagnostic-count (assurance-suite-diagnostic-count suite)
    diagnostic-code (assurance-suite-diagnostic-code suite)
    diagnostic-message (assurance-suite-diagnostic-message suite)
    diagnostic-span (assurance-suite-diagnostic-span suite)
    method (assurance-suite-method suite)
    executed (assurance-suite-executed suite)
    rendered (assurance-report-json
      (assurance-status failed diagnostic-count)
      method
      executed
      executed
      failed
      diagnostic-count
      diagnostic-code
      (vector-get diagnostic-span 1)
      (vector-get diagnostic-span 2)
      diagnostic-message)]
    (do
      (print-string rendered)
      (print-string "\n")
      (if (> failed 0) (exit-runtime-error) (exit-success)))))
(defn run-test-source-json [src]
  (let [program (parse-program src)
    analysis (infer-program-analysis program)
    property-boundary-code (metadata-test-runner-boundary-code program)
    case-check (check-canonical-cases-with-analysis program analysis)]
    (if (> property-boundary-code 0)
      (run-test-source-json-preflight program property-boundary-code 0 0)
      (if (> (vector-get case-check 0) 0)
        (run-test-source-json-preflight
          program
          (vector-get case-check 1)
          (vector-get case-check 2)
          (vector-get case-check 3))
        (run-test-source-json-suite (generate-tests-from-source src))))))
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
      2)))
(defn run-test-source-text [src opts]
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
      (if (> failed 0) 2 (exit-success))))))))
(defn run-test-source [src opts]
  (if (= opts (test-option-json))
    (run-test-source-json src)
    (run-test-source-text src opts)))
(defn review-option-json [] 2)
(defn review-json-source-id [] 200)
(defn run-review-source [src opts] (let [program (parse-program src)] (if (= opts (review-option-json)) (let [review-json (generate-review-schema-json program (review-json-source-id))] (do (print-string review-json) (print-string "\n") (exit-success))) (let [review (generate-review program opts) diagnostics (vector-get review 1) review-title (review-summary-title diagnostics) review-body (review-summary-body diagnostics) review-severity (review-summary-severity diagnostics) review-code-location (review-summary-code-location diagnostics)] (do (print (vector-length diagnostics)) (print-string review-title) (print-string "\n") (print-string review-body) (print-string "\n") (print-string review-severity) (print-string "\n") (print-string review-code-location) (print-string "\n") (exit-success))))))
(defn print-doc-trailers-loop [trailers idx count] (if (>= idx count) 0 (do (print-string (vector-get trailers idx)) (print-string "\n") (print-doc-trailers-loop trailers (+ idx 1) count))))
(defn print-doc-payload [payload] (let [trailers (vector-get payload 3)] (do (print-string (vector-get payload 0)) (print-string "\n") (print-string (vector-get payload 1)) (print-string "\n") (print-string (vector-get payload 2)) (print-string "\n") (print-doc-trailers-loop trailers 0 (vector-length trailers)))))
(defn print-doc-trailer-only [payload] (let [trailers (vector-get payload 3)] (print-doc-trailers-loop trailers 0 (vector-length trailers))))
(defn doc-option-trailer-only [] 10)
(defn doc-option-strict-check [] 11)
(defn invalid-doc-trailer-message [] "invalid doc trailer: expected trailing comment lines")
(defn cli-stderr [msg] (do (print-string (string-concat "error: " msg)) (print-string "\n")))
(defn run-doc-ack-source [src opts] (let [program (parse-program src) ack (generate-doc-ack program "anonymous")] (do (if (= opts (doc-option-trailer-only)) (print-doc-trailer-only ack) (print-doc-payload ack)) (exit-success))))
(defn run-doc-check-source [src opts] (if (and (= opts (doc-option-strict-check)) (= (doc-check-trailer-valid? src) 0)) (do (cli-stderr (invalid-doc-trailer-message)) (exit-compile-error)) (let [program (parse-program src) check (generate-doc-check program "anonymous")] (do (print-doc-payload check) (exit-success)))))
(defn run-fmt-source [src opts] (do (print-string src) (exit-success)))
(defn wasm-size-text [size] (string-concat "wasm-size:" (int-to-string size)))
(defn component-output-boundary-message [] "wasi-component output requires external component packaging")
(defn is-path-sep [path idx] (let [ch (string-char-at path idx)] (if (= ch 47) true (if (= ch 92) true false))))
(defn find-last-path-sep [path idx len last] (if (>= idx len) last (find-last-path-sep path (+ idx 1) len (if (is-path-sep path idx) idx last))))
(defn find-last-char-loop [text idx len char-code last] (if (>= idx len) last (find-last-char-loop text (+ idx 1) len char-code (if (= (string-char-at text idx) char-code) idx last))))
(defn replace-extension [path new-ext] (let [len (string-length path) last-sep (find-last-path-sep path 0 len -1) last-dot (find-last-char-loop path 0 len 46 -1)] (if (> last-dot last-sep) (string-concat (substring path 0 last-dot) new-ext) (string-concat path new-ext))))
(defn default-output-extension [target] (if (= target (compile-target-preview1)) ".wasm" ".component.wasm"))
(defn default-output-path [file-path target] (replace-extension file-path (default-output-extension target)))
(defn compile-file-functions-data-with-cache [file-path cache-ref parse-count-ref] (compile-file-functions-payload-with-cache file-path 12 cache-ref parse-count-ref))
(defn compile-file-functions-data [file-path] (let [cache-ref (ref-new (map-new)) parse-count-ref (ref-new 0)] (compile-file-functions-data-with-cache file-path cache-ref parse-count-ref)))
(defn standalone-preview1-capability-boundary-message [] "unsupported standalone Preview1 runtime capability")
(defn standalone-preview1-input-layout-safe? [src] (< (string-length src) 1024))
(defn standalone-preview1-data-layout-safe? [data] (< (vector-length data) (standalone-data-layout-limit)))
(defn compile-file-wasm-bytes [file-path] (let [pair (compile-file-functions-data file-path) functions (vector-get pair 0) data (vector-get pair 1) unsupported-opcode (standalone-preview1-first-unsupported-opcode functions)] (if (>= unsupported-opcode 0) (vector-new 0) (if (standalone-preview1-data-layout-safe? data) (build-wasm-bytes-wasi-standalone functions data) (vector-new 0)))))
(defn compile-source-wasm-bytes [src] (let [program (parse-program src) pair (compile-program-functions-with-source-base src program 12) functions (vector-get pair 1) data (vector-get pair 2) unsupported-opcode (standalone-preview1-first-unsupported-opcode functions)] (if (>= unsupported-opcode 0) (vector-new 0) (if (standalone-preview1-data-layout-safe? data) (build-wasm-bytes-wasi-standalone functions data) (vector-new 0)))))
(defn run-compile-source [src opts] (if (standalone-preview1-input-layout-safe? src) (let [wasm-bytes (compile-source-wasm-bytes src) wasm-size (vector-length wasm-bytes)] (if (= wasm-size 0) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error)) (do (print-string (wasm-size-text wasm-size)) (print-string "\n") (exit-success)))) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error))))
(defn run-compile-output [file-path output-path opts] (if (file-exists? file-path) (if (= opts (compile-target-preview1)) (let [src (read-file file-path)] (if (standalone-preview1-input-layout-safe? src) (let [wasm-bytes (compile-file-wasm-bytes file-path) wasm-size (vector-length wasm-bytes) summary (wasm-size-text wasm-size)] (if (= wasm-size 0) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error)) (do (write-file-bytes output-path wasm-bytes) (print-string summary) (print-string "\n") (exit-success)))) (do (cli-stderr (standalone-preview1-capability-boundary-message)) (exit-compile-error)))) (do (cli-stderr (component-output-boundary-message)) (exit-compile-error))) (exit-compile-error)))
(defn run-build-output [file-path output-path opts] (run-compile-output file-path output-path opts))
(defn run-parse [file-path opts] (if (file-exists? file-path) (run-parse-source (read-file file-path) opts) (exit-compile-error)))
(defn run-check [file-path opts] (if (file-exists? file-path) (run-check-program (load-check-program file-path) opts) (exit-compile-error)))
(defn run-compile [file-path opts] (if (file-exists? file-path) (run-compile-output file-path (default-output-path file-path opts) opts) (exit-compile-error)))
(defn run-build [file-path opts] (if (file-exists? file-path) (run-build-output file-path (default-output-path file-path opts) opts) (exit-compile-error)))
(defn run-test [file-path opts] (if (file-exists? file-path) (run-test-source (read-file file-path) opts) (exit-compile-error)))
(defn run-review [file-path opts] (if (file-exists? file-path) (run-review-source (read-file file-path) opts) (exit-compile-error)))
(defn run-doc-ack [file-path opts] (if (file-exists? file-path) (run-doc-ack-source (read-file file-path) opts) (exit-compile-error)))
(defn run-doc-check [file-path opts] (if (file-exists? file-path) (run-doc-check-source (read-file file-path) opts) (exit-compile-error)))
(defn run-fmt [file-path opts] (if (file-exists? file-path) (run-fmt-source (read-file file-path) opts) (exit-compile-error)))
(defn run-validate [file-path opts]
  (if (file-exists? file-path)
    (run-validate-source (read-file file-path) opts)
    (do
      (cli-stderr "source file not found")
      (exit-compile-error))))
(defn output-option-flag [arg] (or (string-eq arg "-o") (string-eq arg "--output")))
(defn target-option-flag [arg] (string-eq arg "--target"))
(defn json-option-flag [arg] (string-eq arg "--json"))
(defn format-option-flag [arg] (string-eq arg "--format"))
(defn cli-option-status-ok [] 0)
(defn cli-option-status-invalid-target [] 1)
(defn cli-option-status-missing-value [] 2)
(defn cli-option-status-unsupported-option [] 3)
(defn cli-option-result [status target output-path detail] (let [result (vector-new 4)] (vector-push (vector-push (vector-push (vector-push result status) target) output-path) detail)))
(defn cli-option-result-status [result] (vector-get result 0))
(defn cli-option-result-target [result] (vector-get result 1))
(defn cli-option-result-output-path [result] (vector-get result 2))
(defn cli-option-result-detail [result] (vector-get result 3))
(defn parse-cli-options-loop [idx argc target output-path] (if (>= idx argc) (cli-option-result (cli-option-status-ok) target output-path "") (let [flag (command-line-arg idx)] (if (>= (+ idx 1) argc) (cli-option-result (cli-option-status-missing-value) target output-path flag) (let [flag-value (command-line-arg (+ idx 1))] (if (output-option-flag flag) (parse-cli-options-loop (+ idx 2) argc target flag-value) (if (target-option-flag flag) (let [parsed-target (parse-compile-target-name flag-value)] (if (< parsed-target 0) (cli-option-result (cli-option-status-invalid-target) target output-path flag-value) (parse-cli-options-loop (+ idx 2) argc parsed-target output-path))) (cli-option-result (cli-option-status-unsupported-option) target output-path flag))))))))
(defn parse-cli-options [argc] (parse-cli-options-loop 2 argc (default-compile-target) ""))
(defn review-cli-option-none [] 0)
(defn review-cli-option-invalid [] (- 0 1))
(defn parse-review-cli-option [argc]
  (if (<= argc 2)
    (review-cli-option-none)
    (let [arg2 (command-line-arg 2)]
      (if (and (= argc 3) (json-option-flag arg2))
        (review-option-json)
        (if (= argc 4)
          (if (format-option-flag arg2)
            (if (string-eq (command-line-arg 3) "json")
              (review-option-json)
              (review-cli-option-invalid))
              (review-cli-option-invalid))
            (review-cli-option-invalid))))))
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
(defn parse-test-cli-option [argc]
  (if (<= argc 2)
    0
    (let [arg2 (command-line-arg 2)]
      (if (and (= argc 3) (json-option-flag arg2))
        (test-option-json)
        (if (= argc 4)
          (if (format-option-flag arg2)
            (if (string-eq (command-line-arg 3) "json")
              (test-option-json)
              (check-cli-option-invalid))
            (check-cli-option-invalid))
          (check-cli-option-invalid))))))
(defn validate-option-json [] 1)
(defn validate-option-text [] 2)
(defn validate-option-invalid [] (- 0 1))
(defn validate-options-result-with-identity
  [status manifest-path detail source-path subject source artifact trust lifecycle now]
  (let [result (vector-new 10)
    with-status (push-int-vector-local result status)
    with-path (push-object-vector-local with-status manifest-path)
    with-detail (push-object-vector-local with-path detail)
    with-source (push-object-vector-local with-detail source-path)
    with-subject (push-object-vector-local with-source subject)
    with-source-commit (push-object-vector-local with-subject source)
    with-artifact (push-object-vector-local with-source-commit artifact)
    with-trust (push-object-vector-local with-artifact trust)
    with-lifecycle (push-object-vector-local with-trust lifecycle)]
    (push-object-vector-local with-lifecycle now)))
(defn validate-options-result [status manifest-path detail source-path]
  (validate-options-result-with-identity
    status
    manifest-path
    detail
    source-path
    ""
    ""
    ""
    ""
    ""
    ""))
(defn validate-options-status [result] (vector-get result 0))
(defn validate-options-manifest-path [result] (vector-get result 1))
(defn validate-options-detail [result] (vector-get result 2))
(defn validate-options-source-path [result] (vector-get result 3))
(defn validate-option-review-flag? [flag]
  (or
    (or
      (or
        (string-eq flag "--review-subject-digest")
        (string-eq flag "--review-source-commit"))
      (or
        (string-eq flag "--review-artifact-digest")
        (string-eq flag "--review-trust-store-digest")))
    (or
      (string-eq flag "--review-lifecycle-digest")
      (string-eq flag "--review-now"))))
(defn validate-option-identity-context-valid? [subject source artifact now]
  (if (and
        (= (string-length subject) 0)
        (and
          (= (string-length source) 0)
          (and (= (string-length artifact) 0) (= (string-length now) 0))))
    1
    (if (and
          (> (string-length subject) 0)
          (and
            (> (string-length source) 0)
            (and (> (string-length artifact) 0) (> (string-length now) 0))))
      1
      0)))
(defn validate-review-option-state [status subject source artifact trust lifecycle now]
  (let [result (vector-new 7)
    with-status (push-int-vector-local result status)
    with-subject (push-object-vector-local with-status subject)
    with-source (push-object-vector-local with-subject source)
    with-artifact (push-object-vector-local with-source artifact)
    with-trust (push-object-vector-local with-artifact trust)
    with-lifecycle (push-object-vector-local with-trust lifecycle)]
    (push-object-vector-local with-lifecycle now)))
(defn parse-validate-review-option
  [flag value subject source artifact trust lifecycle now]
  (if (= (string-length value) 0)
    (validate-review-option-state 0 subject source artifact trust lifecycle now)
    (if (string-eq flag "--review-subject-digest")
      (validate-review-option-state 1 value source artifact trust lifecycle now)
      (if (string-eq flag "--review-source-commit")
        (validate-review-option-state 1 subject value artifact trust lifecycle now)
        (if (string-eq flag "--review-artifact-digest")
          (validate-review-option-state 1 subject source value trust lifecycle now)
          (if (string-eq flag "--review-trust-store-digest")
            (validate-review-option-state 1 subject source artifact value lifecycle now)
            (if (string-eq flag "--review-lifecycle-digest")
              (validate-review-option-state 1 subject source artifact trust value now)
              (if (string-eq flag "--review-now")
                (validate-review-option-state 1 subject source artifact trust lifecycle value)
                (validate-review-option-state 0 subject source artifact trust lifecycle now)))))))))
(defn parse-validate-cli-review-branch
  [idx argc source-path manifest-path source-seen format-seen flag value subject source artifact trust lifecycle now]
  (let [review-result
      (parse-validate-review-option
        flag
        value
        subject
        source
        artifact
        trust
        lifecycle
        now)]
    (if (= (vector-get review-result 0) 1)
      (parse-validate-cli-options-loop
        (+ idx 2)
        argc
        source-path
        manifest-path
        source-seen
        format-seen
        (vector-get review-result 1)
        (vector-get review-result 2)
        (vector-get review-result 3)
        (vector-get review-result 4)
        (vector-get review-result 5)
        (vector-get review-result 6))
      (validate-options-result (validate-option-invalid) manifest-path flag source-path))))
(defn parse-validate-cli-option-step
  [idx argc source-path manifest-path source-seen format-seen flag value subject source artifact trust lifecycle now]
  (if (string-eq flag "--emit-manifest")
    (if (> (string-length value) 0)
      (parse-validate-cli-options-loop
        (+ idx 2)
        argc
        source-path
        value
        source-seen
        format-seen
        subject
        source
        artifact
        trust
        lifecycle
        now)
      (validate-options-result (validate-option-invalid) manifest-path flag source-path))
    (if (format-option-flag flag)
      (if (string-eq value "json")
        (parse-validate-cli-options-loop
          (+ idx 2)
          argc
          source-path
          manifest-path
          source-seen
          1
          subject
          source
          artifact
          trust
          lifecycle
          now)
        (if (string-eq value "text")
          (parse-validate-cli-options-loop
            (+ idx 2)
            argc
            source-path
            manifest-path
            source-seen
            2
            subject
            source
            artifact
            trust
            lifecycle
            now)
          (validate-options-result (validate-option-invalid) manifest-path value source-path)))
      (if (string-eq flag "--source")
        (if (> (string-length value) 0)
          (parse-validate-cli-options-loop
            (+ idx 2)
            argc
            value
            manifest-path
            1
            format-seen
            subject
            source
            artifact
            trust
            lifecycle
            now)
          (validate-options-result (validate-option-invalid) manifest-path flag source-path))
        (parse-validate-cli-review-branch
          idx
          argc
          source-path
          manifest-path
          source-seen
          format-seen
          flag
          value
          subject
          source
          artifact
          trust
          lifecycle
          now)))))
(defn parse-validate-cli-options-loop
  [idx argc source-path manifest-path source-seen format-seen subject source artifact trust lifecycle now]
  (if (>= idx argc)
    (if (and (= source-seen 1) (> format-seen 0))
      (if (= (validate-option-identity-context-valid? subject source artifact now) 1)
        (validate-options-result-with-identity
          format-seen
          manifest-path
          ""
          source-path
          subject
          source
          artifact
          trust
          lifecycle
          now)
        (validate-options-result
          (validate-option-invalid)
          manifest-path
          "review identity requires --review-subject-digest --review-source-commit --review-artifact-digest --review-now"
          source-path))
      (validate-options-result
        (validate-option-invalid)
        manifest-path
        "validate requires --source <file> --format text|json"
        source-path))
    (let [flag (command-line-arg idx)]
      (if (or
            (or
              (or (string-eq flag "--source") (format-option-flag flag))
              (string-eq flag "--emit-manifest"))
            (validate-option-review-flag? flag))
        (if (>= (+ idx 1) argc)
          (validate-options-result (validate-option-invalid) manifest-path flag source-path)
          (parse-validate-cli-option-step
            idx
            argc
            source-path
            manifest-path
            source-seen
            format-seen
            flag
            (command-line-arg (+ idx 1))
            subject
            source
            artifact
            trust
            lifecycle
            now))
        (validate-options-result (validate-option-invalid) manifest-path flag source-path)))))
(defn parse-validate-cli-options [argc]
  (parse-validate-cli-options-loop 1 argc "" "" 0 0 "" "" "" "" "" ""))
(defn parse-validate-cli-option [argc]
  (let [result (parse-validate-cli-options argc)]
    (if (or
          (= (validate-options-status result) (validate-option-json))
          (= (validate-options-status result) (validate-option-text)))
      (validate-options-status result)
      (validate-option-invalid))))
(defn run-validate-command [argc]
  (let [options (parse-validate-cli-options argc)]
    (if (or
          (= (validate-options-status options) (validate-option-json))
          (= (validate-options-status options) (validate-option-text)))
      (run-validate (validate-options-source-path options) options)
      (do
        (cli-stderr (string-concat "validate option error: " (validate-options-detail options)))
        (exit-compile-error)))))
(defn doc-cli-option-none [] 0)
(defn doc-cli-option-invalid [] (- 0 1))
(defn parse-doc-cli-option [argc cmd-name]
  (if (<= argc 2)
    (doc-cli-option-none)
    (let [arg2 (command-line-arg 2)]
      (if (string-eq cmd-name "doc-ack")
        (if (and (= argc 3) (string-eq arg2 "--trailer")) (doc-option-trailer-only) (doc-cli-option-invalid))
        (if (string-eq cmd-name "doc-check")
          (if (and (= argc 3) (string-eq arg2 "--strict")) (doc-option-strict-check) (doc-cli-option-invalid))
          (doc-cli-option-none))))))
(defn run-command-with-doc-option [cmd-name file-path doc-option]
  (let [cmd-id (arg-parse cmd-name)]
    (if (= cmd-id (cmd-doc-ack))
      (run-doc-ack file-path doc-option)
      (if (= cmd-id (cmd-doc-check))
        (run-doc-check file-path doc-option)
        (run-command cmd-name file-path doc-option)))))
(defn run-command [cmd-name file-path opts]
  (let [cmd-id (arg-parse cmd-name)]
    (if (= cmd-id (cmd-parse))
      (run-parse file-path opts)
      (if (= cmd-id (cmd-check))
        (run-check file-path opts)
        (if (= cmd-id (cmd-compile))
          (run-compile file-path opts)
          (if (= cmd-id (cmd-build))
            (run-build file-path opts)
            (if (= cmd-id (cmd-test))
              (run-test file-path opts)
              (if (= cmd-id (cmd-review))
                (run-review file-path opts)
                (if (= cmd-id (cmd-doc-ack))
                  (run-doc-ack file-path opts)
                  (if (= cmd-id (cmd-doc-check))
                    (run-doc-check file-path opts)
                    (if (= cmd-id (cmd-fmt))
                      (run-fmt file-path opts)
                      (if (= cmd-id (cmd-validate))
                        (run-validate file-path opts)
                        (exit-compile-error)))))))))))))
(defn run-command-with-cli-options [cmd-name file-path result] (let [target (cli-option-result-target result) output-path (cli-option-result-output-path result)] (if (> (string-length output-path) 0) (if (string-eq cmd-name "compile") (run-compile-output file-path output-path target) (if (string-eq cmd-name "build") (run-build-output file-path output-path target) (run-command cmd-name file-path target))) (run-command cmd-name file-path target))))
(defn compile-or-build-command [cmd-name] (or (string-eq cmd-name "compile") (string-eq cmd-name "build")))
(defn run-main-command [argc cmd-name file-path]
  (if (string-eq cmd-name "validate")
    (run-validate-command argc)
    (if (and (compile-or-build-command cmd-name) (> argc 2))
    (let [options (parse-cli-options argc)]
      (if (= (cli-option-result-status options) (cli-option-status-ok))
        (run-command-with-cli-options cmd-name file-path options)
        (exit-compile-error)))
    (if (and (string-eq cmd-name "review") (> argc 2))
      (let [review-option (parse-review-cli-option argc)]
        (if (>= review-option 0)
          (run-command cmd-name file-path review-option)
          (exit-compile-error)))
      (if (and (string-eq cmd-name "check") (> argc 2))
        (let [check-option (parse-check-cli-option argc)]
          (if (>= check-option 0)
            (run-command cmd-name file-path check-option)
            (exit-compile-error)))
        (if (and (string-eq cmd-name "test") (> argc 2))
          (let [test-option (parse-test-cli-option argc)]
            (if (>= test-option 0)
              (run-command cmd-name file-path test-option)
              (exit-compile-error)))
          (if (and (> argc 2) (or (string-eq cmd-name "doc-ack") (string-eq cmd-name "doc-check")))
            (let [doc-option (parse-doc-cli-option argc cmd-name)]
              (if (>= doc-option 0)
                (run-command-with-doc-option cmd-name file-path doc-option)
                (exit-compile-error)))
            (run-command cmd-name file-path (default-compile-target)))))))))
(defn exit-main [code] (do (proc-exit code) 0))
(defn main []
  (let [argc (command-line-args)]
    (if (= argc 0)
      (exit-main (exit-compile-error))
      (let [cmd-name (command-line-arg 0)
        file-path (if (> argc 1) (command-line-arg 1) "")]
        (exit-main (run-main-command argc cmd-name file-path))))))
