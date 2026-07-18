(module Types.MetadataMigration)
(import Syntax.AST)
(import Syntax.Parser)
(import Types.Type)
(import Types.TypeInfer)
(import Types.TypeInferCore)
(import Types.TypeInferAssertions)

;; legacy metadata を canonical form へ silent conversion せず分類する。
;; row は [diagnostic-code, disposition, directive-start, directive-end, owner-hash]。
;; 先頭 4 フィールドは既存 summary/span consumer との互換を維持する。disposition は
;; 1=docs-only :example, 2=:assert, 3=:property/:postcondition,
;; 4=manual review を表す。

(defn legacy-example-code [] 2001)
(defn legacy-invariant-code [] 2002)
(defn ambiguous-legacy-code [] 2003)
(defn legacy-doc-example-disposition [] 1)
(defn legacy-assertion-disposition [] 2)
(defn legacy-property-disposition [] 3)
(defn legacy-manual-disposition [] 4)

(defn legacy-migration-message [code disposition]
  (if (= code (ambiguous-legacy-code))
    "legacy :example は silent conversion できません。manual review が必要です"
    (if (= code (legacy-invariant-code))
      "legacy :invariant は :property / :postcondition への移行候補です"
      (if (= disposition (legacy-assertion-disposition))
        "Bool legacy :example は strict :assert への移行候補です"
        "legacy :example は docs-only :example として保持する候補です"))))

(defn legacy-migration-row [code disposition start end owner]
  (let [base (vector-push-single-rooted
      (vector-push-quad-rooted (vector-new 4) code disposition start end)
      owner)]
    (do
      (root_push base)
      (let [message (legacy-migration-message code disposition)]
        (do
          (root_push message)
          (let [result (vector-push base message)]
            (do
              (root_pop)
              (root_pop)
              result)))))))

(defn legacy-example-row-for-expression [expression env counter start end owner]
  (let [result (infer-expr expression env (subst-new) counter)]
    (if (= (result-failed result) 1)
      (legacy-migration-row
        (ambiguous-legacy-code)
        (legacy-manual-disposition)
        start
        end
        owner)
      (let [resolved (apply-subst (result-subst result) (result-type result))
        tag (type-tag resolved)]
        (if (= tag (ty-var))
          (legacy-migration-row
            (ambiguous-legacy-code)
            (legacy-manual-disposition)
            start
            end
            owner)
          (if (= tag (ty-con))
            (if (= (type-name resolved) (hash-bool))
              (legacy-migration-row
                (legacy-example-code)
                (legacy-assertion-disposition)
                start
                end
                owner)
              (legacy-migration-row
                (legacy-example-code)
                (legacy-doc-example-disposition)
                start
                end
                owner))
            (legacy-migration-row
              (legacy-example-code)
              (legacy-doc-example-disposition)
              start
              end
              owner)))))))

(defn legacy-example-expressions-loop
  [expressions idx count env counter start end owner result]
  (if (>= idx count)
    result
    (let [row (legacy-example-row-for-expression
        (vector-get expressions idx)
        env
        counter
        start
        end
        owner)
      next-result (vector-push-single-rooted result row)]
      (do
        (root_push next-result)
        (let [parsed (legacy-example-expressions-loop
            expressions
            (+ idx 1)
            count
            env
            counter
            start
            end
            owner
            next-result)]
          (do
            (root_pop)
            parsed))))))

(defn legacy-example-form [form env counter owner result]
  (let [example-text (vector-get form 1)
    start (if (> (vector-length form) 2) (vector-get form 2) 0)
    end (if (> (vector-length form) 3) (vector-get form 3) 0)
    expressions (parse-program example-text)]
    (do
      (root_push expressions)
      (let [parsed (legacy-example-expressions-loop
          expressions
          0
          (vector-length expressions)
          env
          counter
          start
          end
          owner
          result)]
        (do
          (root_pop)
          parsed)))))

(defn legacy-form-loop [forms idx count env counter owner result]
  (if (>= idx count)
    result
    (let [form (vector-get forms idx)
      kind (vector-get form 0)
      next-result (if (= kind (contract-form-example))
        (legacy-example-form form env counter owner result)
        (if (= kind (contract-form-invariant))
          (vector-push-single-rooted
            result
            (legacy-migration-row
              (legacy-invariant-code)
              (legacy-property-disposition)
              (if (> (vector-length form) 2) (vector-get form 2) 0)
              (if (> (vector-length form) 3) (vector-get form 3) 0)
              owner))
          result))]
      (do
        (root_push next-result)
        (let [parsed (legacy-form-loop
            forms
            (+ idx 1)
            count
            env
            counter
            owner
            next-result)]
          (do
            (root_pop)
            parsed))))))

(defn legacy-defn [decl env counter result]
  (let [forms (defn-ordered-forms decl)]
    (if (= forms 0)
      result
      (legacy-form-loop
        forms
        0
        (vector-length forms)
        env
        counter
        (vector-get decl 1)
        result))))

(defn legacy-program-loop
  [program idx count env counter result]
  (if (>= idx count)
    result
    (let [decl (vector-get program idx)
      tag (vector-get decl 0)
      next-result (if (= tag (ast-defn))
        (legacy-defn decl env counter result)
        (if (= tag (ast-private))
          (legacy-program-loop
            (vector-push-single-rooted (vector-new 1) (vector-get decl 1))
            0
            1
            env
            counter
            result)
          (if (= tag (ast-module-decl))
            (legacy-module decl result)
            result)))]
      (do
        (root_push next-result)
        (let [parsed (legacy-program-loop
            program
            (+ idx 1)
            count
            env
            counter
            next-result)]
          (do
            (root_pop)
            parsed))))))

(defn legacy-module [module-node result]
  (let [module-program (canonical-module-program module-node)
    analysis (infer-program-analysis module-program)
    env (infer-program-analysis-env analysis)
    counter (typeinfer-make-alias-aware-counter module-program)]
    (legacy-program-loop
      module-program
      0
      (vector-length module-program)
      env
      counter
      result)))

(defn classify-legacy-contracts [program]
  (let [analysis (infer-program-analysis program)
    env (infer-program-analysis-env analysis)
    counter (typeinfer-make-alias-aware-counter program)]
    (legacy-program-loop
      program
      0
      (vector-length program)
      env
      counter
      (vector-new 0))))

(defn legacy-code-text [code]
  (string-concat "LS" (int-to-string code)))

(defn legacy-disposition-text [disposition]
  (if (= disposition (legacy-doc-example-disposition))
    "docs-only-example"
    (if (= disposition (legacy-assertion-disposition))
      "assertion"
      (if (= disposition (legacy-property-disposition))
        "property-postcondition"
        "manual-review"))))

(defn legacy-migration-row-text [row]
  (string-concat
    (legacy-code-text (vector-get row 0))
    (string-concat ":" (legacy-disposition-text (vector-get row 1)))))

(defn legacy-migration-summary-loop [rows idx count]
  (if (>= idx count)
    ""
    (let [piece (legacy-migration-row-text (vector-get rows idx))
      rest (legacy-migration-summary-loop rows (+ idx 1) count)]
      (if (= (string-length rest) 0)
        piece
        (string-concat piece (string-concat "," rest))))))

(defn legacy-migration-summary [rows]
  (let [count (vector-length rows)]
    (if (= count 0)
      ""
      (string-concat
        "migration:"
        (string-concat
          (int-to-string count)
          (string-concat "," (legacy-migration-summary-loop rows 0 count)))))))
