(module Types.TypeInferAssertions)
(import Syntax.AST)
(import Syntax.Parser)
(import Types.Type)
(import Types.TypeInfer)
(import Types.TypeInferCore)
;; TypeInfer の既存 AST/環境を使った canonical :assert の狭い型検査。predicate は関数引数を暗黙に束縛せず、実行せずに Bool を確認する。
(defn canonical-assertion-type-error-code [] 1001)
(defn canonical-assertion-non-bool-code [] 1002)
(defn canonical-assertion-empty-code [] 2004)
(defn canonical-assertion-vacuous-code [] 2005)
(defn canonical-case-type-error-code [] 1001)
(defn canonical-case-value-error-code [] 1002)
(defn canonical-case-empty-code [] 2006)
(defn canonical-property-type-error-code [] 1001)
(defn canonical-property-non-bool-code [] 1002)
(defn canonical-property-empty-code [] 2007)
(defn assertion-check-state [diagnostic-count first-error-code]
  (vector-push-pair-rooted
    (vector-new 2)
    diagnostic-count
    first-error-code))
(defn property-space? [ch]
  (if (or (= ch 32) (= ch 9))
    1
    (if (or (= ch 10) (= ch 13)) 1 0)))
(defn property-skip-space [src idx len]
  (if (>= idx len)
    idx
    (if (= (property-space? (string-char-at src idx)) 1)
      (property-skip-space src (+ idx 1) len)
      idx)))
(defn property-find-substring-loop [src needle idx len needle-len]
  (if (> (+ idx needle-len) len)
    -1
    (if (string-eq (substring src idx (+ idx needle-len)) needle)
      idx
      (property-find-substring-loop src needle (+ idx 1) len needle-len))))
(defn property-find-substring [src needle]
  (property-find-substring-loop src needle 0 (string-length src) (string-length needle)))
(defn property-balanced-expression-end [src idx len depth]
  (if (>= idx len)
    -1
    (let [ch (string-char-at src idx)]
      (if (= ch 40)
        (property-balanced-expression-end src (+ idx 1) len (+ depth 1))
        (if (= ch 41)
          (if (= depth 1)
            (+ idx 1)
            (property-balanced-expression-end src (+ idx 1) len (- depth 1)))
          (property-balanced-expression-end src (+ idx 1) len depth))))))
(defn property-atom-expression-end [src idx len]
  (if (>= idx len)
    idx
    (let [ch (string-char-at src idx)]
      (if (or
        (= (property-space? ch) 1)
        (or (= ch 41) (= ch 93)))
        idx
        (property-atom-expression-end src (+ idx 1) len)))))
(defn property-postcondition-text [payload]
  (let [marker (property-find-substring payload ":postcondition")
    payload-len (string-length payload)]
    (if (< marker 0)
      ""
      (let [expression-start (property-skip-space payload (+ marker (string-length ":postcondition")) payload-len)]
        (if (>= expression-start payload-len)
          ""
          (let [expression-end (if (= (string-char-at payload expression-start) 40)
              (property-balanced-expression-end payload expression-start payload-len 0)
              (property-atom-expression-end payload expression-start payload-len))]
            (if (<= expression-end expression-start)
              ""
              (substring payload expression-start expression-end))))))))
(defn property-binder-source-loop [payload idx close len result]
  (let [name-start (property-skip-space payload idx len)]
    (if (>= name-start close)
      result
      (let [name-end (property-atom-expression-end payload name-start len)
        type-start (property-skip-space payload name-end len)
        type-end (if (= (string-char-at payload type-start) 40)
          (property-balanced-expression-end payload type-start len 0)
          (property-atom-expression-end payload type-start len))
        binder (if (or (= name-end name-start) (<= type-end type-start))
          ""
          (string-concat
            "(: "
            (string-concat
              (substring payload name-start name-end)
              (string-concat " " (string-concat (substring payload type-start type-end) ")")))))]
        (if (= (string-length binder) 0)
          result
          (property-binder-source-loop
            payload
            (property-skip-space payload type-end len)
            close
            len
            (if (= (string-length result) 0)
              binder
              (string-concat result (string-concat " " binder)))))))))
(defn property-probe-parameter-source [payload]
  (let [open (property-find-substring payload "[")
    len (string-length payload)
    close (property-find-substring-loop payload "]" (+ open 1) len 1)]
    (if (or (< open 0) (< close 0))
      "[result]"
      (let [binders (property-binder-source-loop payload (+ open 1) close len "")]
        (if (= (string-length binders) 0)
          "[result]"
          (string-concat "[" (string-concat binders " result]")))))))
(defn property-probe-return-type [ty]
  (if (= (ty-tag ty) (ty-fun))
    (property-probe-return-type (type-fun-ret ty))
    ty))
(defn property-probe-predicate [program] (let [decl (vector-get program 0)] (vector-get decl (+ 3 (vector-get decl 2)))))
(defn check-property-predicate [payload expression reject-vacuous reject-unreachable]
  (if (= (string-length expression) 0)
    (canonical-property-type-error-code)
    (do
      (root_push payload)
      (root_push expression)
      (let [probe-source (string-concat "(defn __lsharp_property_probe " (string-concat (property-probe-parameter-source payload) (string-concat " " (string-concat expression ")"))))]
        (do
          (root_push probe-source)
          (let [probe-program (parse-program probe-source)]
            (do
              (root_push probe-program)
              (let [analysis (infer-program-analysis probe-program)]
                (do
                  (root_push analysis)
                  (let [diagnostic-count (infer-program-analysis-diagnostic-count analysis)
                    result (if (> diagnostic-count 0)
                      (canonical-property-type-error-code)
                      (let [predicate (property-probe-predicate probe-program)
                        resolved (property-probe-return-type (infer-program-analysis-type analysis))
                        type-code (if (and (= (ty-tag resolved) (ty-con)) (= (ty-name resolved) (hash-bool))) 0 (canonical-property-non-bool-code))
                        boolean-result (statically-boolean-result predicate)]
                        (if (and (= reject-vacuous 1) (= boolean-result 1))
                          (canonical-assertion-vacuous-code)
                          (if (and (= reject-unreachable 1) (= boolean-result 2))
                            (canonical-assertion-vacuous-code)
                            type-code))))]
                    (do
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      (root_pop)
                      result)))))))))))
(defn check-property-postcondition [payload] (let [expression (property-postcondition-text payload)] (if (= (string-length expression) 0) (canonical-property-empty-code) (if (string-eq expression "true") (canonical-assertion-vacuous-code) (check-property-predicate payload expression 1 0)))))
(defn check-property-preconditions-loop [payload idx close len]
  (let [expression-start (property-skip-space payload idx len)]
    (if (>= expression-start close)
      0
      (do
        (root_push payload)
        (let [expression-end (if (= (string-char-at payload expression-start) 40)
            (property-balanced-expression-end payload expression-start len 0)
            (property-atom-expression-end payload expression-start len))]
          (if (<= expression-end expression-start)
            (do
              (root_pop)
              (canonical-property-type-error-code))
            (let [expression (substring payload expression-start expression-end)]
              (do
                (root_push expression)
                (let [code (if (string-eq expression "false") (canonical-assertion-vacuous-code) (check-property-predicate payload expression 0 1))]
                  (if (> code 0)
                    (do
                      (root_pop)
                      (root_pop)
                      code)
                    (let [result (check-property-preconditions-loop
                        payload
                        expression-end
                        close
                        len)]
                      (do
                        (root_pop)
                        (root_pop)
                        result))))))))))))
(defn check-property-precondition [payload]
  (let [marker (property-find-substring payload ":precondition")
    len (string-length payload)]
    (if (< marker 0)
      0
      (let [bracket-start (property-skip-space payload (+ marker 13) len)]
        (if (or (>= bracket-start len) (!= (string-char-at payload bracket-start) 91))
          (canonical-property-type-error-code)
          (let [close (property-find-substring-loop payload "]" (+ bracket-start 1) len 1)]
            (if (< close 0)
              (canonical-property-type-error-code)
              (check-property-preconditions-loop payload (+ bracket-start 1) close len))))))))
(defn defn-metadata [decl]
  (let [param-count (vector-get decl 2)
    body-end (+ 4 param-count)
    decl-length (vector-length decl)
    signature-offset
      (if (< body-end decl-length)
        (let [candidate (vector-get decl body-end)]
          (if (= candidate 0)
            0
            (if (= (vector-get candidate 0) (ast-defn-signature)) 1 0)))
        0)
    metadata-index (+ body-end signature-offset)]
    (if (< metadata-index decl-length)
      (vector-get decl metadata-index)
      0)))
(defn defn-ordered-forms [decl]
  (let [metadata (defn-metadata decl)]
    (if (= metadata 0)
      0
      (if (> (vector-length metadata) 5)
      (vector-get metadata 5)
      0))))
(defn canonical-unprivate-decl [decl]
  (if (= (vector-get decl 0) (ast-private))
    (canonical-unprivate-decl (vector-get decl 1))
    decl))
;; module の flattened body を infer-program-analysis が受け取れる vector に戻す。
(defn canonical-module-program-loop [module-node idx count result]
  (if (>= idx count)
    result
    (let [raw-decl (vector-get module-node (+ 3 idx))
      decl (canonical-unprivate-decl raw-decl)
      next-result (vector-push-single-rooted result decl)]
      (do
        (root_push next-result)
        (let [parsed (canonical-module-program-loop module-node (+ idx 1) count next-result)]
          (do
            (root_pop)
            parsed))))))
(defn canonical-module-program [module-node]
  (let [count (if (> (vector-length module-node) 2) (vector-get module-node 2) 0)
    result (vector-new 0)]
    (do
      (root_push module-node)
      (root_push result)
      (let [parsed (canonical-module-program-loop module-node 0 count result)]
        (do
          (root_pop)
          (root_pop)
          parsed)))))
(defn assertion-contains-param-loop [predicate decl idx count]
  (if (>= idx count)
    0
    (if (= (ast-contains-var predicate (vector-get decl (+ 3 idx))) 1)
      1
      (assertion-contains-param-loop predicate decl (+ idx 1) count))))
;; operator の name-hash は Parser の 31-fold hash と一致させる。
(defn static-comparison-operator? [operator]
  (if (= operator 61) 1
    (if (= operator 1952) 1
      (if (= operator 1084) 1
        (if (= operator 60) 1
          (if (= operator 62) 1
            (if (= operator 1921) 1
              (if (= operator 1983) 1 0))))))))
(defn static-comparison-result? [operator left right]
  (if (or (= operator 61) (= operator 1952))
    (if (= left right) 1 2)
    (if (= operator 1084)
      (if (!= left right) 1 2)
      (if (= operator 60)
        (if (< left right) 1 2)
        (if (= operator 62)
          (if (> left right) 1 2)
          (if (= operator 1921)
            (if (<= left right) 1 2)
            (if (= operator 1983)
              (if (>= left right) 1 2)
              0)))))))
(defn statically-integer-comparison? [predicate expected]
  (let [tag (vector-get predicate 0)]
    (if (= tag (ast-ann))
      (statically-integer-comparison? (vector-get predicate 1) expected)
      (if (= tag (ast-apply))
        (let [callee (vector-get predicate 1)
          arg-count (vector-get predicate 2)]
          (if (= arg-count 2)
            (if (= (vector-get callee 0) (ast-var))
              (if (= (static-comparison-operator? (vector-get callee 1)) 1)
                (if (= (vector-get (vector-get predicate 3) 0) (ast-lit-int))
                  (if (= (vector-get (vector-get predicate 4) 0) (ast-lit-int))
                    (if (= (static-comparison-result?
                        (vector-get callee 1)
                        (vector-get (vector-get predicate 3) 1)
                        (vector-get (vector-get predicate 4) 1)) expected) 1 0)
                    0)
                  0)
                0)
              0)
            0))
        0))))
(defn statically-true-integer-comparison? [predicate] (statically-integer-comparison? predicate 1))
(defn statically-false-integer-comparison? [predicate] (statically-integer-comparison? predicate 2))
(defn statically-false-bool? [predicate] (let [tag (vector-get predicate 0)] (if (= tag (ast-ann)) (statically-false-bool? (vector-get predicate 1)) (if (and (= tag (ast-lit-bool)) (= (vector-get predicate 1) 0)) 1 0))))
(defn static-logic-and-hash [] 96727)
(defn static-logic-or-hash [] 3555)
(defn static-logic-not-hash [] 109267)
(defn static-boolean-and-result [left right]
  (if (= left 2)
    2
    (if (= right 2)
      2
      (if (= left 1)
        right
        (if (= right 1) left 0)))))
(defn static-boolean-or-result [left right]
  (if (= left 1)
    1
    (if (= right 1)
      1
      (if (= left 2)
        right
        (if (= right 2) left 0)))))
(defn statically-boolean-result [predicate]
  (let [tag (vector-get predicate 0)]
    (if (= tag (ast-ann))
      (statically-boolean-result (vector-get predicate 1))
      (if (= tag (ast-lit-bool))
        (if (= (vector-get predicate 1) 1) 1 2)
        (if (= tag (ast-apply))
          (let [callee (vector-get predicate 1)
            arg-count (vector-get predicate 2)]
            (if (= (vector-get callee 0) (ast-var))
              (let [operator (vector-get callee 1)]
                (if (= operator (static-logic-not-hash))
                  (if (= arg-count 1)
                    (let [operand-result (statically-boolean-result (vector-get predicate 3))]
                      (if (= operand-result 1) 2 (if (= operand-result 2) 1 0)))
                    0)
                  (if (= arg-count 2)
                    (let [left (vector-get predicate 3)
                      right (vector-get predicate 4)]
                      (if (= operator (static-logic-and-hash))
                        (static-boolean-and-result
                          (statically-boolean-result left)
                          (statically-boolean-result right))
                        (if (= operator (static-logic-or-hash))
                          (static-boolean-or-result
                            (statically-boolean-result left)
                            (statically-boolean-result right))
                          (if (= (statically-true-integer-comparison? predicate) 1)
                            1
                            (if (= (statically-false-integer-comparison? predicate) 1) 2 0)))))
                    0)))
              0))
          0)))))
(defn check-assertion-predicate [predicate decl env counter]
  (if (or
    (and (= (vector-get predicate 0) (ast-lit-bool)) (= (vector-get predicate 1) 1))
    (= (statically-true-integer-comparison? predicate) 1))
    (canonical-assertion-vacuous-code)
    (if (= (assertion-contains-param-loop
      predicate
      decl
      0
      (vector-get decl 2)) 1)
      (canonical-assertion-type-error-code)
      (let [result (infer-expr predicate env (subst-new) counter)]
        (if (= (result-failed result) 1)
          (canonical-assertion-type-error-code)
          (let [resolved (apply-subst (result-subst result) (result-type result))]
            (if (= (type-tag resolved) (ty-con))
              (if (= (type-name resolved) (hash-bool))
                0
                (canonical-assertion-non-bool-code))
              (canonical-assertion-non-bool-code))))))))
(defn check-assertion-predicates-loop
  [predicates idx count decl env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [code (check-assertion-predicate
        (vector-get predicates idx)
        decl
        env
        counter)
      next-count (if (> code 0) (+ diagnostic-count 1) diagnostic-count)
      next-first-error-code
        (if (= first-error-code 0) code first-error-code)]
      (check-assertion-predicates-loop
        predicates
        (+ idx 1)
        count
        decl
        env
        counter
        next-count
        next-first-error-code))))
(defn check-assertion-form [form decl env counter diagnostic-count first-error-code]
  (if (= (vector-get form 0) 3)
    (let [predicates (vector-get form 1)]
      (if (= (vector-length predicates) 0)
        (assertion-check-state
          (+ diagnostic-count 1)
          (if (= first-error-code 0)
            (canonical-assertion-empty-code)
            first-error-code))
        (check-assertion-predicates-loop
          predicates
          0
          (vector-length predicates)
          decl
          env
          counter
          diagnostic-count
          first-error-code)))
    (assertion-check-state diagnostic-count first-error-code)))
(defn check-assertion-forms-loop
  [forms idx count decl env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [state (check-assertion-form
        (vector-get forms idx)
        decl
        env
        counter
        diagnostic-count
        first-error-code)]
      (check-assertion-forms-loop
        forms
        (+ idx 1)
        count
        decl
        env
        counter
        (vector-get state 0)
        (vector-get state 1)))))
(defn check-defn-assertions [decl env counter]
  (let [forms (defn-ordered-forms decl)]
    (if (= forms 0)
      (assertion-check-state 0 0)
      (check-assertion-forms-loop
        forms
        0
        (vector-length forms)
        decl
        env
        counter
        0
        0))))
;; canonical :case は owner の引数や result を暗黙に束縛しない。
(defn canonical-case-primitive-kind [resolved]
  (if (= (type-tag resolved) (ty-con))
    (if (= (type-name resolved) (hash-int))
      1
      (if (= (type-name resolved) (hash-bool)) 2 0))
    0))
(defn check-case-expectation [pair decl env counter]
  (let [actual (vector-get pair 0)
    expected (vector-get pair 1)
    parameter-count (vector-get decl 2)]
    (if (or
      (= (assertion-contains-param-loop actual decl 0 parameter-count) 1)
      (= (assertion-contains-param-loop expected decl 0 parameter-count) 1))
      (canonical-case-type-error-code)
      (let [actual-result (infer-expr actual env (subst-new) counter)]
        (if (= (result-failed actual-result) 1)
          (canonical-case-type-error-code)
          (let [actual-type (apply-subst
              (result-subst actual-result)
              (result-type actual-result))
            actual-kind (canonical-case-primitive-kind actual-type)
            expected-result (infer-expr expected env (subst-new) counter)]
            (if (= (result-failed expected-result) 1)
              (canonical-case-type-error-code)
              (let [expected-type (apply-subst
                  (result-subst expected-result)
                  (result-type expected-result))
                expected-kind (canonical-case-primitive-kind expected-type)]
                (if (or (= actual-kind 0) (= expected-kind 0))
                  (canonical-case-value-error-code)
                  (if (= actual-kind expected-kind)
                    0
                    (canonical-case-value-error-code)))))))))))
(defn check-case-expectations-loop
  [expectations idx count decl env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [code (check-case-expectation
        (vector-get expectations idx)
        decl
        env
        counter)
      next-count (if (> code 0) (+ diagnostic-count 1) diagnostic-count)
      next-first-error-code
        (if (= first-error-code 0) code first-error-code)]
      (check-case-expectations-loop
        expectations
        (+ idx 1)
        count
        decl
        env
        counter
        next-count
        next-first-error-code))))
(defn check-case-form [form decl env counter diagnostic-count first-error-code]
  (if (= (vector-get form 0) (contract-form-case))
    (let [expectations (vector-get form 1)]
      (if (= (vector-length expectations) 0)
        (assertion-check-state
          (+ diagnostic-count 1)
          (if (= first-error-code 0)
            (canonical-case-empty-code)
            first-error-code))
        (check-case-expectations-loop
          expectations
          0
          (vector-length expectations)
          decl
          env
          counter
          diagnostic-count
          first-error-code)))
    (assertion-check-state diagnostic-count first-error-code)))
(defn check-case-forms-loop
  [forms idx count decl env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [state (check-case-form
        (vector-get forms idx)
        decl
        env
        counter
        diagnostic-count
        first-error-code)]
      (check-case-forms-loop
        forms
        (+ idx 1)
        count
        decl
        env
        counter
        (vector-get state 0)
        (vector-get state 1)))))
(defn check-defn-cases [decl env counter]
  (let [forms (defn-ordered-forms decl)]
    (if (= forms 0)
      (assertion-check-state 0 0)
      (check-case-forms-loop
        forms
        0
        (vector-length forms)
        decl
        env
        counter
        0
        0))))
(defn check-module-program-loop
  [program idx count env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [decl (vector-get program idx)]
      (do
        (root_push decl)
        (let [tag (vector-get decl 0)
          state (if (= tag (ast-defn))
            (check-defn-assertions decl env counter)
            (if (= tag (ast-module-decl))
              (check-module-assertions decl)
              (assertion-check-state 0 0)))]
          (do
            (root_push state)
            (let [next-count (+ diagnostic-count (vector-get state 0))
              state-first-code (vector-get state 1)
              next-first-code
                (if (= first-error-code 0) state-first-code first-error-code)
              result (check-module-program-loop
                program
                (+ idx 1)
                count
                env
                counter
                next-count
                next-first-code)]
              (do
                (root_pop)
                (root_pop)
                result))))))))
(defn check-module-assertions [module-node]
  (let [module-program (canonical-module-program module-node)]
    (let [analysis (infer-program-analysis module-program)
      counter (typeinfer-make-alias-aware-counter module-program)
      env (infer-program-analysis-env analysis)]
      (check-module-program-loop
        module-program
        0
        (vector-length module-program)
        env
        counter
        0
        0))))
(defn check-program-assertions-loop
  [program idx count env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [decl (vector-get program idx)]
      (do
        (root_push decl)
        (let [tag (vector-get decl 0)
          state (if (= tag (ast-defn))
            (check-defn-assertions decl env counter)
            (if (= tag (ast-module-decl))
              (check-module-assertions decl)
              (if (= tag (ast-private))
                (let [inner (canonical-unprivate-decl decl)
                  inner-tag (vector-get inner 0)]
                  (if (= inner-tag (ast-defn))
                    (check-defn-assertions inner env counter)
                    (if (= inner-tag (ast-module-decl))
                      (check-module-assertions inner)
                      (assertion-check-state 0 0))))
                (assertion-check-state 0 0))))]
          (do
            (root_push state)
            (let [next-count (+ diagnostic-count (vector-get state 0))
              state-first-code (vector-get state 1)
              next-first-code
                (if (= first-error-code 0) state-first-code first-error-code)
              result (check-program-assertions-loop
                program
                (+ idx 1)
                count
                env
                counter
                next-count
                next-first-code)]
              (do
                (root_pop)
                (root_pop)
                result))))))))
(defn check-canonical-assertions-with-analysis [program analysis]
  (let [counter (typeinfer-make-alias-aware-counter program)
    env (infer-program-analysis-env analysis)]
    (check-program-assertions-loop
      program
      0
      (vector-length program)
      env
      counter
      0
      0)))
(defn check-canonical-assertions [program]
  (let [analysis (infer-program-analysis program)]
    (check-canonical-assertions-with-analysis program analysis)))
(defn check-case-module-program-loop
  [program idx count env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [decl (vector-get program idx)]
      (do
        (root_push decl)
        (let [tag (vector-get decl 0)
          state (if (= tag (ast-defn))
            (check-defn-cases decl env counter)
            (if (= tag (ast-module-decl))
              (check-case-module decl)
              (assertion-check-state 0 0)))]
          (do
            (root_push state)
            (let [next-count (+ diagnostic-count (vector-get state 0))
              state-first-code (vector-get state 1)
              next-first-code
                (if (= first-error-code 0) state-first-code first-error-code)
              result (check-case-module-program-loop
                program
                (+ idx 1)
                count
                env
                counter
                next-count
                next-first-code)]
              (do
                (root_pop)
                (root_pop)
                result))))))))
(defn check-case-module [module-node]
  (let [module-program (canonical-module-program module-node)]
    (let [analysis (infer-program-analysis module-program)
      counter (typeinfer-make-alias-aware-counter module-program)
      env (infer-program-analysis-env analysis)]
      (check-case-module-program-loop
        module-program
        0
        (vector-length module-program)
        env
        counter
        0
        0))))
(defn check-case-program-loop
  [program idx count env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [decl (vector-get program idx)]
      (do
        (root_push decl)
        (let [tag (vector-get decl 0)
          state (if (= tag (ast-defn))
            (check-defn-cases decl env counter)
            (if (= tag (ast-module-decl))
              (check-case-module decl)
              (if (= tag (ast-private))
                (let [inner (canonical-unprivate-decl decl)
                  inner-tag (vector-get inner 0)]
                  (if (= inner-tag (ast-defn))
                    (check-defn-cases inner env counter)
                    (if (= inner-tag (ast-module-decl))
                      (check-case-module inner)
                      (assertion-check-state 0 0))))
                (assertion-check-state 0 0))))]
          (do
            (root_push state)
            (let [next-count (+ diagnostic-count (vector-get state 0))
              state-first-code (vector-get state 1)
              next-first-code
                (if (= first-error-code 0) state-first-code first-error-code)
              result (check-case-program-loop
                program
                (+ idx 1)
                count
                env
                counter
                next-count
                next-first-code)]
              (do
                (root_pop)
                (root_pop)
                result))))))))
(defn check-canonical-cases-with-analysis [program analysis]
  (let [counter (typeinfer-make-alias-aware-counter program)
    env (infer-program-analysis-env analysis)]
    (check-case-program-loop
      program
      0
      (vector-length program)
      env
      counter
      0
      0)))
(defn check-canonical-cases [program]
  (let [analysis (infer-program-analysis program)]
    (check-canonical-cases-with-analysis program analysis)))
(defn property-binders-empty? [payload] (if (string-eq (property-probe-parameter-source payload) "[result]") 1 0))
(defn property-cases-zero? [payload] (let [marker (property-find-substring payload ":cases") len (string-length payload)] (if (< marker 0) 0 (let [start (property-skip-space payload (+ marker 6) len) end (property-atom-expression-end payload start len)] (if (<= end start) 0 (if (= (parse-int-from-str payload start end 0) 0) 1 0))))))
(defn property-cases-invalid? [payload] (let [marker (property-find-substring payload ":cases") len (string-length payload)] (if (< marker 0) 0 (let [start (property-skip-space payload (+ marker 6) len)] (if (< start len) (let [ch (string-char-at payload start)] (if (= ch 45) 1 (if (< ch 48) 1 (if (> ch 57) 1 0)))) 0)))))
(defn check-property-form [form diagnostic-count first-error-code]
  (if (= (vector-get form 0) (contract-form-property))
    (do
      (root_push form)
      (let [payload (if (> (vector-length form) 1) (vector-get form 1) "")
        structural-code (if (or (= (property-binders-empty? payload) 1) (or (= (property-cases-zero? payload) 1) (= (property-cases-invalid? payload) 1))) (canonical-property-empty-code) 0)
        precondition-code (check-property-precondition payload)
        code (if (> structural-code 0) structural-code (if (> precondition-code 0) precondition-code (check-property-postcondition payload)))
        next-count (if (> code 0) (+ diagnostic-count 1) diagnostic-count)
        next-first-code (if (= first-error-code 0) code first-error-code)
        state (assertion-check-state next-count next-first-code)]
        (do
          (root_pop)
          state)))
    (assertion-check-state diagnostic-count first-error-code)))
(defn check-property-forms-loop
  [forms idx count diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (do
      (root_push forms)
      (let [form (vector-get forms idx)]
        (do
          (root_push form)
          (let [state (check-property-form
              form
              diagnostic-count
              first-error-code)]
            (do
              (root_push state)
              (let [result (check-property-forms-loop
                  forms
                  (+ idx 1)
                  count
                  (vector-get state 0)
                  (vector-get state 1))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn check-defn-properties [decl]
  (let [forms (defn-ordered-forms decl)]
    (if (= forms 0)
      (assertion-check-state 0 0)
      (check-property-forms-loop
        forms
        0
        (vector-length forms)
        0
        0))))
(defn check-property-module [module-node]
  (let [module-program (canonical-module-program module-node)]
    (do
      (root_push module-program)
      (let [result (check-property-program-loop
          module-program
          0
          (vector-length module-program)
          0
          0)]
        (do
          (root_pop)
          result)))))
(defn check-property-decl [decl]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (check-defn-properties decl)
      (if (= tag (ast-private))
        (check-property-decl (vector-get decl 1))
        (if (= tag (ast-module-decl))
          (check-property-module decl)
          (assertion-check-state 0 0))))))

(defn check-property-program-loop
  [program idx count diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (do
      (root_push program)
      (let [decl (vector-get program idx)]
        (do
          (root_push decl)
          (let [state (check-property-decl decl)]
            (do
              (root_push state)
              (let [result (check-property-program-loop
                  program
                  (+ idx 1)
                  count
                  (+ diagnostic-count (vector-get state 0))
                  (if (= first-error-code 0)
                    (vector-get state 1)
                    first-error-code))]
                (do
                  (root_pop)
                  (root_pop)
                  (root_pop)
                  result)))))))))
(defn check-canonical-properties-with-analysis [program analysis]
  (check-property-program-loop
    program
    0
    (vector-length program)
    0
    0))

(defn check-canonical-properties [program]
  (check-canonical-properties-with-analysis
    program
    (infer-program-analysis program)))
