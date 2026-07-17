(module Types.TypeInferAssertions)
(import Syntax.AST)
(import Types.Type)
(import Types.TypeInfer)
(import Types.TypeInferCore)

;; TypeInfer の既存 AST/環境を使った canonical :assert の狭い型検査。
;; predicate は関数引数を暗黙に束縛せず、実行せずに Bool を確認する。

(defn canonical-assertion-type-error-code [] 1001)
(defn canonical-assertion-non-bool-code [] 1002)

(defn assertion-check-state [diagnostic-count first-error-code]
  (vector-push-pair-rooted
    (vector-new 2)
    diagnostic-count
    first-error-code))

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

(defn assertion-contains-param-loop [predicate decl idx count]
  (if (>= idx count)
    0
    (if (= (ast-contains-var predicate (vector-get decl (+ 3 idx))) 1)
      1
      (assertion-contains-param-loop predicate decl (+ idx 1) count))))

(defn check-assertion-predicate [predicate decl env counter]
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
            (canonical-assertion-non-bool-code)))))))

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
      (check-assertion-predicates-loop
        predicates
        0
        (vector-length predicates)
        decl
        env
        counter
        diagnostic-count
        first-error-code))
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

(defn check-program-assertions-loop
  [program idx count env counter diagnostic-count first-error-code]
  (if (>= idx count)
    (assertion-check-state diagnostic-count first-error-code)
    (let [decl (vector-get program idx)
      state (if (= (vector-get decl 0) (ast-defn))
        (check-defn-assertions decl env counter)
        (assertion-check-state 0 0))
      next-count (+ diagnostic-count (vector-get state 0))
      state-first-code (vector-get state 1)
      next-first-code
        (if (= first-error-code 0) state-first-code first-error-code)]
      (check-program-assertions-loop
        program
        (+ idx 1)
        count
        env
        counter
        next-count
        next-first-code))))

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
