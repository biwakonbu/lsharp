(module Tools.Validation.Evidence)
(import Tools.Validation.IntentSource)
(import Tools.Lsp.JsonRpc)

;; Rust の EvidenceForm を将来の selfhost parser から渡せる registry 境界。
;; form: [15, payload, span-start, span-end]
;; payload: [id, subject, method, outcome, runner, target, source-commit,
;;           artifact-digest, cases, seed, generator, shrinks, coverage,
;;           producer, tool-version, timestamp, independence]
;; coverage: [[bucket, count] ...]

(defn source-evidence-form-kind [] 15)

(defn source-evidence-error-malformed [] 1)
(defn source-evidence-error-invalid-id [] 2)
(defn source-evidence-error-duplicate-id [] 3)
(defn source-evidence-error-empty-field [] 4)
(defn source-evidence-error-invalid-subject [] 5)
(defn source-evidence-error-missing-subject [] 6)
(defn source-evidence-error-invalid-method [] 7)
(defn source-evidence-error-invalid-outcome [] 8)
(defn source-evidence-error-invalid-independence [] 9)
(defn source-evidence-error-duplicate-coverage [] 10)
(defn source-evidence-error-invalid-sampling [] 11)
(defn source-evidence-error-invalid-edge [] 12)
(defn source-evidence-error-registry-required [] 13)

(defn source-evidence-error-record [code field value start end related-start related-end]
  (let [base (vector-push-quad-rooted-v3 (vector-new 1) code field value start)
    with-end (vector-push-single-rooted-v3 base end)
    with-related-start (vector-push-single-rooted-v3 with-end related-start)]
    (vector-push-single-rooted-v3 with-related-start related-end)))

(defn source-evidence-error [code field value start end]
  (source-evidence-error-record code field value start end -1 -1))

(defn source-evidence-error-related [code field value start end related-start related-end]
  (source-evidence-error-record code field value start end related-start related-end))

(defn source-evidence-error-code [error] (vector-get error 0))
(defn source-evidence-error-field [error] (vector-get error 1))
(defn source-evidence-error-value [error] (vector-get error 2))
(defn source-evidence-error-start [error] (vector-get error 3))
(defn source-evidence-error-end [error] (vector-get error 4))
(defn source-evidence-error-related-start [error] (vector-get error 5))
(defn source-evidence-error-related-end [error] (vector-get error 6))

(defn source-evidence-payload
  [id subject method outcome runner target source-commit artifact-digest cases seed generator shrinks coverage producer tool-version timestamp independence]
  (let [base (vector-push-quad-rooted-v3 (vector-new 1) id subject method outcome)
    with-runner (vector-push-single-rooted-v3 base runner)
    with-target (vector-push-single-rooted-v3 with-runner target)
    with-source (vector-push-single-rooted-v3 with-target source-commit)
    with-artifact (vector-push-single-rooted-v3 with-source artifact-digest)
    with-cases (vector-push-single-rooted-v3 with-artifact cases)
    with-seed (vector-push-single-rooted-v3 with-cases seed)
    with-generator (vector-push-single-rooted-v3 with-seed generator)
    with-shrinks (vector-push-single-rooted-v3 with-generator shrinks)
    with-coverage (vector-push-single-rooted-v3 with-shrinks coverage)
    with-producer (vector-push-single-rooted-v3 with-coverage producer)
    with-tool-version (vector-push-single-rooted-v3 with-producer tool-version)
    with-timestamp (vector-push-single-rooted-v3 with-tool-version timestamp)]
    (vector-push-single-rooted-v3 with-timestamp independence)))

(defn source-evidence-form [payload start end]
  (vector-push-quad-rooted-v3 (vector-new 4) (source-evidence-form-kind) payload start end))

(defn source-evidence-form-payload [form] (vector-get form 1))
(defn source-evidence-record-id [form] (vector-get (source-evidence-form-payload form) 0))
(defn source-evidence-record-subject [form] (vector-get (source-evidence-form-payload form) 1))
(defn source-evidence-record-method [form] (vector-get (source-evidence-form-payload form) 2))
(defn source-evidence-record-outcome [form] (vector-get (source-evidence-form-payload form) 3))
(defn source-evidence-record-runner [form] (vector-get (source-evidence-form-payload form) 4))
(defn source-evidence-record-target [form] (vector-get (source-evidence-form-payload form) 5))
(defn source-evidence-record-source-commit [form] (vector-get (source-evidence-form-payload form) 6))
(defn source-evidence-record-artifact-digest [form] (vector-get (source-evidence-form-payload form) 7))
(defn source-evidence-record-cases [form] (vector-get (source-evidence-form-payload form) 8))
(defn source-evidence-record-seed [form] (vector-get (source-evidence-form-payload form) 9))
(defn source-evidence-record-generator [form] (vector-get (source-evidence-form-payload form) 10))
(defn source-evidence-record-shrinks [form] (vector-get (source-evidence-form-payload form) 11))
(defn source-evidence-record-coverage [form] (vector-get (source-evidence-form-payload form) 12))
(defn source-evidence-record-producer [form] (vector-get (source-evidence-form-payload form) 13))
(defn source-evidence-record-tool-version [form] (vector-get (source-evidence-form-payload form) 14))
(defn source-evidence-record-timestamp [form] (vector-get (source-evidence-form-payload form) 15))
(defn source-evidence-record-independence [form] (vector-get (source-evidence-form-payload form) 16))
(defn source-evidence-record-start [form] (vector-get form 2))
(defn source-evidence-record-end [form] (vector-get form 3))

(defn source-evidence-subject-kind [subject]
  (if (= (source-wire-valid? subject (source-node-intent)) 1)
    (source-node-intent)
    (if (= (source-wire-valid? subject (source-node-claim)) 1)
      (source-node-claim)
      (if (= (source-wire-valid? subject (source-edge-tested-by)) 1)
        (source-edge-tested-by)
        0))))

(defn source-evidence-empty-field [payload]
  (if (= (string-length (vector-get payload 0)) 0)
    "id"
    (if (= (string-length (vector-get payload 1)) 0)
      "subject"
      (if (= (string-length (vector-get payload 2)) 0)
        "method"
        (if (= (string-length (vector-get payload 3)) 0)
          "outcome"
          (if (= (string-length (vector-get payload 4)) 0)
            "runner"
            (if (= (string-length (vector-get payload 5)) 0)
              "target"
              (if (= (string-length (vector-get payload 6)) 0)
                "source-commit"
                (if (= (string-length (vector-get payload 7)) 0)
                  "artifact-digest"
                  (if (= (string-length (vector-get payload 10)) 0)
                    "generator"
                    (if (= (string-length (vector-get payload 13)) 0)
                      "producer"
                      (if (= (string-length (vector-get payload 14)) 0)
                        "tool-version"
                        (if (= (string-length (vector-get payload 15)) 0)
                          "timestamp"
                          (if (= (string-length (vector-get payload 16)) 0)
                            "independence"
                            ""))))))))))))))

(defn source-evidence-method-valid? [value]
  (if
    (or
      (or
        (or (string-eq value "example") (string-eq value "case"))
        (or (string-eq value "assert") (string-eq value "property")))
      (or
        (or (string-eq value "production") (string-eq value "reference"))
        (or (string-eq value "proof") (string-eq value "review"))))
    1
    0))

(defn source-evidence-outcome-valid? [value]
  (if
    (or
      (or (string-eq value "pass") (string-eq value "fail"))
      (or
        (string-eq value "contradicted")
        (or (string-eq value "unknown") (string-eq value "stale"))))
    1
    0))

(defn source-evidence-independence-valid? [value]
  (if
    (or
      (string-eq value "same-author")
      (or
        (string-eq value "independent-review")
        (string-eq value "external-observation")))
    1
    0))

(defn source-evidence-shrinks-valid-loop [shrinks idx len]
  (if (>= idx len)
    1
    (if (< (vector-get shrinks idx) 0)
      0
      (source-evidence-shrinks-valid-loop shrinks (+ idx 1) len))))

(defn source-evidence-coverage-has-bucket-loop [coverage bucket idx len]
  (if (>= idx len)
    0
    (let [entry (vector-get coverage idx)]
      (if (and (> (vector-length entry) 1) (string-eq (vector-get entry 0) bucket))
        1
        (source-evidence-coverage-has-bucket-loop coverage bucket (+ idx 1) len)))))

(defn source-evidence-coverage-valid-loop [coverage idx len]
  (if (>= idx len)
    (source-result 1 0)
    (let [entry (vector-get coverage idx)]
      (if (< (vector-length entry) 2)
        (source-result 0 (source-evidence-error (source-evidence-error-invalid-sampling) "coverage" "" -1 -1))
        (let [bucket (vector-get entry 0)
          count (vector-get entry 1)]
          (if (= (string-length bucket) 0)
            (source-result 0 (source-evidence-error (source-evidence-error-empty-field) "coverage" "" -1 -1))
            (if (< count 0)
              (source-result 0 (source-evidence-error (source-evidence-error-invalid-sampling) "coverage" bucket -1 -1))
              (if (= (source-evidence-coverage-has-bucket-loop coverage bucket 0 idx) 1)
                (source-result 0 (source-evidence-error (source-evidence-error-duplicate-coverage) "coverage" bucket -1 -1))
                (source-evidence-coverage-valid-loop coverage (+ idx 1) len)))))))))

(defn source-evidence-form-result [form nodes]
  (if (< (vector-length form) 4)
    (source-result 0 (source-evidence-error (source-evidence-error-malformed) "form" "" -1 -1))
    (let [kind (vector-get form 0)
      payload (vector-get form 1)
      start (vector-get form 2)
      end (vector-get form 3)]
      (if (or (!= kind (source-evidence-form-kind)) (< (vector-length payload) 17))
        (source-result 0 (source-evidence-error (source-evidence-error-malformed) "form" "" start end))
        (let [empty-field (source-evidence-empty-field payload)
          id (vector-get payload 0)
          subject (vector-get payload 1)
          method (vector-get payload 2)
          outcome (vector-get payload 3)
          subject-kind (source-evidence-subject-kind subject)]
          (if (> (string-length empty-field) 0)
            (source-result 0 (source-evidence-error (source-evidence-error-empty-field) empty-field "" start end))
            (if (= (source-wire-valid? id (source-edge-supports)) 0)
              (source-result 0 (source-evidence-error (source-evidence-error-invalid-id) "id" id start end))
              (if (= subject-kind 0)
                (source-result 0 (source-evidence-error (source-evidence-error-invalid-subject) "subject" subject start end))
                (if (and
                      (or (= subject-kind (source-node-intent)) (= subject-kind (source-node-claim)))
                      (= (source-node-id-exists? nodes subject) 0))
                  (source-result 0 (source-evidence-error (source-evidence-error-missing-subject) "subject" subject start end))
                  (if (= (source-evidence-method-valid? method) 0)
                    (source-result 0 (source-evidence-error (source-evidence-error-invalid-method) "method" method start end))
                    (if (= (source-evidence-outcome-valid? outcome) 0)
                      (source-result 0 (source-evidence-error (source-evidence-error-invalid-outcome) "outcome" outcome start end))
                      (if (= (source-evidence-independence-valid? (vector-get payload 16)) 0)
                        (source-result 0 (source-evidence-error (source-evidence-error-invalid-independence) "independence" (vector-get payload 16) start end))
                        (if (< (vector-get payload 8) 0)
                          (source-result 0 (source-evidence-error (source-evidence-error-invalid-sampling) "cases" "" start end))
                          (if (= (source-evidence-shrinks-valid-loop (vector-get payload 11) 0 (vector-length (vector-get payload 11))) 0)
                            (source-result 0 (source-evidence-error (source-evidence-error-invalid-sampling) "shrinks" "" start end))
                            (let [coverage-result (source-evidence-coverage-valid-loop (vector-get payload 12) 0 (vector-length (vector-get payload 12)))]
                              (if (= (source-result-status coverage-result) 0)
                                coverage-result
                                (source-result 1 form)))))))))))))))))

(defn source-evidence-registry-new [] (vector-new 0))
(defn source-evidence-registry-length [registry] (vector-length registry))
(defn source-evidence-registry-record [registry idx] (vector-get registry idx))

(defn source-evidence-id-exists-loop [registry id idx len]
  (if (>= idx len)
    0
    (if (string-eq (source-evidence-record-id (vector-get registry idx)) id)
      1
      (source-evidence-id-exists-loop registry id (+ idx 1) len))))

(defn source-evidence-id-exists? [registry id]
  (source-evidence-id-exists-loop registry id 0 (vector-length registry)))

(defn source-evidence-find-loop [registry id idx len]
  (if (>= idx len)
    0
    (let [evidence-record (vector-get registry idx)]
      (if (string-eq (source-evidence-record-id evidence-record) id)
        evidence-record
        (source-evidence-find-loop registry id (+ idx 1) len)))))

(defn source-evidence-register-form [registry nodes form]
  (let [parsed (source-evidence-form-result form nodes)]
    (if (= (source-result-status parsed) 0)
      parsed
      (let [evidence-record (source-result-value parsed)
        id (source-evidence-record-id evidence-record)]
        (if (= (source-evidence-id-exists? registry id) 1)
          (let [first (source-evidence-find-loop registry id 0 (vector-length registry))]
            (source-result 0
              (source-evidence-error-related
                (source-evidence-error-duplicate-id)
                "id"
                id
                (source-evidence-record-start evidence-record)
                (source-evidence-record-end evidence-record)
                (source-evidence-record-start first)
                (source-evidence-record-end first))))
          (source-result 1 (vector-push-single-rooted-v3 registry evidence-record)))))))

(defn source-evidence-edge-result [relation evidence-id claim-id registry nodes start end]
  (if (or (= relation (source-edge-supports)) (= relation (source-edge-contradicts)))
    (if (= (source-wire-valid? evidence-id (source-edge-supports)) 0)
      (source-result 0 (source-evidence-error (source-evidence-error-invalid-id) "observation" evidence-id start end))
      (if (= (source-wire-valid? claim-id (source-node-claim)) 0)
        (source-result 0 (source-evidence-error (source-evidence-error-invalid-id) "claim" claim-id start end))
        (if (= (source-node-id-exists? nodes claim-id) 0)
          (source-result 0 (source-evidence-error (source-evidence-error-missing-subject) "claim" claim-id start end))
          (if (= (source-evidence-id-exists? registry evidence-id) 0)
            (source-result 0 (source-evidence-error (source-evidence-error-registry-required) "observation" evidence-id start end))
            (source-result 1 (source-edge-record relation evidence-id claim-id start end))))))
    (source-result 0 (source-evidence-error (source-evidence-error-invalid-edge) "relation" "" start end))))

(defn source-evidence-append-forms [forms idx len registry nodes]
  (if (>= idx len)
    (source-result 1 registry)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)]
      (if (= kind (source-evidence-form-kind))
        (let [parsed (source-evidence-register-form registry nodes form)]
          (if (= (source-result-status parsed) 0)
            parsed
            (source-evidence-append-forms
              forms
              (+ idx 1)
              len
              (source-result-value parsed)
              nodes)))
        (source-evidence-append-forms forms (+ idx 1) len registry nodes)))))

(defn source-evidence-collect-children [decl idx len registry nodes]
  (if (>= idx len)
    (source-result 1 registry)
    (let [child (vector-get decl idx)
      parsed (source-evidence-collect-decl child registry nodes)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-evidence-collect-children
          decl
          (+ idx 1)
          len
          (source-result-value parsed)
          nodes)))))

(defn source-evidence-collect-decl [decl registry nodes]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (source-ordered-forms decl)]
        (source-evidence-append-forms forms 0 (vector-length forms) registry nodes))
      (if (= tag (ast-private))
        (source-evidence-collect-decl (vector-get decl 1) registry nodes)
        (if (= tag (ast-module-decl))
          (source-evidence-collect-children decl 5 (vector-length decl) registry nodes)
          (if (= tag (ast-impldef))
            (source-evidence-collect-children decl 4 (vector-length decl) registry nodes)
            (source-result 1 registry)))))))

(defn source-evidence-collect-program-loop [program idx len registry nodes]
  (if (>= idx len)
    (source-result 1 registry)
    (let [parsed (source-evidence-collect-decl
        (vector-get program idx)
        registry
        nodes)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-evidence-collect-program-loop
          program
          (+ idx 1)
          len
          (source-result-value parsed)
          nodes)))))

(defn source-evidence-registry-from-program [program]
  (let [nodes-result (source-collect-nodes program)]
    (if (= (source-result-status nodes-result) 0)
      nodes-result
      (source-evidence-collect-program-loop
        program
        0
        (vector-length program)
        (source-evidence-registry-new)
        (source-result-value nodes-result)))))

(defn source-evidence-graph [nodes edges registry]
  (let [base (source-graph nodes edges)]
    (vector-push-single-rooted-v3 base registry)))

(defn source-evidence-graph-registry [graph]
  (vector-get graph 2))

(defn validation-json-object-wrap [body]
  (string-concat "{" (string-concat body "}")))
(defn validation-json-array-wrap [body]
  (string-concat "[" (string-concat body "]")))
(defn validation-json-append [out piece]
  (if (= (string-length out) 0) piece (string-concat out (string-concat "," piece))))
(defn validation-json-field [name value-json]
  (string-concat "\"" (string-concat name (string-concat "\":" value-json))))
(defn validation-json-string-literal [value]
  (string-concat "\"" (string-concat (json-escape-string value) "\"")))
(defn validation-json-string-field [name value]
  (validation-json-field name (validation-json-string-literal value)))
(defn validation-json-int-field [name value]
  (validation-json-field name (int-to-string value)))
(defn validation-json-array-field [name value-json]
  (validation-json-field name value-json))
(defn validation-json-object-field [name value-json]
  (validation-json-field name value-json))

;; native x86 では object/string を含む多引数再帰を state へ畳み、
;; serializer の各 step を一引数の tail call として保持する。
(defn validation-source-manifest-json-state [state]
  state)

;; source graph を Rust の version 1 manifest serializer と同じ wire shape へ投影する。
(defn validation-source-node-kind-text [kind]
  (if (= kind (source-node-intent)) "intent"
    (if (= kind (source-node-claim)) "claim"
      (if (= kind (source-node-assumption)) "assumption" "open-question"))))

(defn validation-source-edge-relation-text [relation]
  (if (= relation (source-edge-motivates)) "motivates"
    (if (= relation (source-edge-constrained-by)) "constrained-by"
      (if (= relation (source-edge-tested-by)) "tested-by"
        (if (= relation (source-edge-supports)) "supports" "contradicts")))))

(defn validation-source-id-json [wire-id]
  (let [len (string-length wire-id)
    colon (source-find-char wire-id 58 0 len)
    slash (if (>= colon 0) (source-find-char wire-id 47 (+ colon 1) len) -1)
    ns-text (if (and (> colon 0) (> slash colon)) (substring wire-id (+ colon 1) slash) "")
    key-text (if (and (> slash 0) (< slash len)) (substring wire-id (+ slash 1) len) "")
    fields0 ""
    fields1 (validation-json-append fields0 (validation-json-string-field "namespace" ns-text))
    fields2 (validation-json-append fields1 (validation-json-string-field "key" key-text))]
    (validation-json-object-wrap fields2)))

(defn validation-source-span-json [start end]
  (let [fields0 ""
    fields1 (validation-json-append fields0 (validation-json-int-field "start" start))
    fields2 (validation-json-append fields1 (validation-json-int-field "end" end))]
    (validation-json-object-wrap fields2)))

(defn validation-source-node-json [node]
  (let [fields0 ""
    fields1 (validation-json-append fields0
      (validation-json-string-field "kind" (validation-source-node-kind-text (source-node-kind node))))
    fields2 (validation-json-append fields1
      (validation-json-string-field "namespace"
        (let [wire-id (source-node-id node)
          colon (source-find-char wire-id 58 0 (string-length wire-id))]
          (substring wire-id (+ colon 1)
            (source-find-char wire-id 47 (+ colon 1) (string-length wire-id))))))
    fields3 (validation-json-append fields2
      (validation-json-string-field "key"
        (let [wire-id (source-node-id node)
          colon (source-find-char wire-id 58 0 (string-length wire-id))
          slash (source-find-char wire-id 47 (+ colon 1) (string-length wire-id))]
          (substring wire-id (+ slash 1) (string-length wire-id)))))
    fields4 (validation-json-append fields3 (validation-json-string-field "text" (source-node-text node)))
    fields5 (validation-json-append fields4
      (validation-json-object-field "span"
        (validation-source-span-json (source-node-start node) (source-node-end node))))]
    (validation-json-object-wrap fields5)))

(defn validation-source-nodes-json-state-loop [state]
  (let [nodes (vector-get state 0)
    idx (vector-get state 1)
    len (vector-get state 2)
    out (vector-get state 3)]
    (if (>= idx len)
      out
      (let [next-out (validation-json-append out (validation-source-node-json (vector-get nodes idx)))
        next-indexed-state (vector-set-at-rooted-v3 state 1 (+ idx 1))
        next-state (vector-set-at-rooted-v3 next-indexed-state 3 next-out)]
        (validation-source-nodes-json-state-loop next-state)))))

(defn validation-source-int-array-json-state-loop [state]
  (let [items (vector-get state 0)
    idx (vector-get state 1)
    len (vector-get state 2)
    out (vector-get state 3)]
    (if (>= idx len)
      out
      (let [next-out (validation-json-append out (int-to-string (vector-get items idx)))
        next-indexed-state (vector-set-at-rooted-v3 state 1 (+ idx 1))
        next-state (vector-set-at-rooted-v3 next-indexed-state 3 next-out)]
        (validation-source-int-array-json-state-loop next-state)))))

(defn validation-source-coverage-json-state-loop [state]
  (let [coverage (vector-get state 0)
    idx (vector-get state 1)
    len (vector-get state 2)
    out (vector-get state 3)]
    (if (>= idx len)
      out
      (let [entry (vector-get coverage idx)
        bucket (vector-get entry 0)
        count (vector-get entry 1)
        next-out (validation-json-append out (validation-json-int-field bucket count))
        next-indexed-state (vector-set-at-rooted-v3 state 1 (+ idx 1))
        next-state (vector-set-at-rooted-v3 next-indexed-state 3 next-out)]
        (validation-source-coverage-json-state-loop next-state)))))

(defn validation-source-subject-kind-text [subject]
  (let [kind (source-evidence-subject-kind subject)]
    (if (= kind (source-node-intent)) "intent"
      (if (= kind (source-node-claim)) "claim" "contract"))))

(defn validation-source-subject-json [subject]
  (let [fields0 (validation-json-string-field "kind" (validation-source-subject-kind-text subject))
    id-object-fields (string-concat
      (validation-json-string-field "namespace"
        (let [wire-id subject
          colon (source-find-char wire-id 58 0 (string-length wire-id))
          slash (source-find-char wire-id 47 (+ colon 1) (string-length wire-id))]
          (substring wire-id (+ colon 1) slash)))
      (string-concat ","
        (validation-json-string-field "key"
          (let [wire-id subject
            colon (source-find-char wire-id 58 0 (string-length wire-id))
            slash (source-find-char wire-id 47 (+ colon 1) (string-length wire-id))]
            (substring wire-id (+ slash 1) (string-length wire-id))))))]
    (validation-json-object-wrap (string-concat fields0 (string-concat "," id-object-fields)))))

(defn validation-source-evidence-json [evidence-record]
  (let [id (source-evidence-record-id evidence-record)
    fields0 ""
    fields1 (validation-json-append fields0
      (validation-json-string-field "namespace"
        (let [colon (source-find-char id 58 0 (string-length id))
          slash (source-find-char id 47 (+ colon 1) (string-length id))]
          (substring id (+ colon 1) slash))))
    fields2 (validation-json-append fields1
      (validation-json-string-field "key"
        (let [colon (source-find-char id 58 0 (string-length id))
          slash (source-find-char id 47 (+ colon 1) (string-length id))]
          (substring id (+ slash 1) (string-length id)))))
    fields3 (validation-json-append fields2 (validation-json-string-field "method" (source-evidence-record-method evidence-record)))
    fields4 (validation-json-append fields3
      (validation-json-object-field "subject" (validation-source-subject-json (source-evidence-record-subject evidence-record))))
    fields5 (validation-json-append fields4 (validation-json-string-field "outcome" (source-evidence-record-outcome evidence-record)))
    sampling0 (validation-json-string-field "runner" (source-evidence-record-runner evidence-record))
    sampling1 (validation-json-append sampling0 (validation-json-string-field "target" (source-evidence-record-target evidence-record)))
    sampling2 (validation-json-append sampling1 (validation-json-string-field "source_commit" (source-evidence-record-source-commit evidence-record)))
    sampling3 (validation-json-append sampling2 (validation-json-string-field "artifact_digest" (source-evidence-record-artifact-digest evidence-record)))
    sample-fields0 (validation-json-int-field "cases" (source-evidence-record-cases evidence-record))
    sample-fields1 (validation-json-append sample-fields0 (validation-json-int-field "seed" (source-evidence-record-seed evidence-record)))
    sample-fields2 (validation-json-append sample-fields1 (validation-json-string-field "generator" (source-evidence-record-generator evidence-record)))
    sample-fields3 (validation-json-append sample-fields2
      (validation-json-array-field "shrinks"
        (validation-json-array-wrap
          (validation-source-int-array-json-state-loop
            (validation-source-manifest-json-state
              (vector-push-single-rooted-v3
                (vector-push-single-rooted-v3
                  (vector-push-single-rooted-v3
                    (vector-push-single-rooted-v3
                      (vector-new 4)
                      (source-evidence-record-shrinks evidence-record))
                    0)
                  (vector-length (source-evidence-record-shrinks evidence-record)))
                ""))))))
    sample-fields4 (validation-json-append sample-fields3
      (validation-json-object-field "coverage"
        (validation-json-object-wrap
          (validation-source-coverage-json-state-loop
            (validation-source-manifest-json-state
              (vector-push-single-rooted-v3
                (vector-push-single-rooted-v3
                  (vector-push-single-rooted-v3
                    (vector-push-single-rooted-v3
                      (vector-new 4)
                      (source-evidence-record-coverage evidence-record))
                    0)
                  (vector-length (source-evidence-record-coverage evidence-record)))
                ""))))))
    execution-fields (string-concat sampling3
      (string-concat "," (validation-json-object-field "sampling" (validation-json-object-wrap sample-fields4))))
    fields6 (validation-json-append fields5 (validation-json-object-field "execution" (validation-json-object-wrap execution-fields)))
    provenance0 (validation-json-string-field "producer" (source-evidence-record-producer evidence-record))
    provenance1 (validation-json-append provenance0 (validation-json-string-field "tool_version" (source-evidence-record-tool-version evidence-record)))
    provenance2 (validation-json-append provenance1 (validation-json-string-field "timestamp" (source-evidence-record-timestamp evidence-record)))
    fields7 (validation-json-append fields6 (validation-json-object-field "provenance" (validation-json-object-wrap provenance2)))
    fields8 (validation-json-append fields7 (validation-json-string-field "independence" (source-evidence-record-independence evidence-record)))]
    (validation-json-object-wrap fields8)))

(defn validation-source-evidence-json-state-loop [state]
  (let [registry (vector-get state 0)
    idx (vector-get state 1)
    len (vector-get state 2)
    out (vector-get state 3)]
    (if (>= idx len)
      out
      (let [next-out (validation-json-append out (validation-source-evidence-json (vector-get registry idx)))
        next-indexed-state (vector-set-at-rooted-v3 state 1 (+ idx 1))
        next-state (vector-set-at-rooted-v3 next-indexed-state 3 next-out)]
        (validation-source-evidence-json-state-loop next-state)))))

(defn validation-source-edge-json [edge]
  (let [relation (source-edge-kind edge)
    left (source-edge-left edge)
    right (source-edge-right edge)
    fields0 (validation-json-string-field "relation" (validation-source-edge-relation-text relation))
    fields1
      (if (= relation (source-edge-motivates))
        (validation-json-append fields0 (validation-json-object-field "intent" (validation-source-id-json left)))
        (if (= relation (source-edge-constrained-by))
          (validation-json-append fields0 (validation-json-object-field "claim" (validation-source-id-json left)))
          (if (= relation (source-edge-tested-by))
            (validation-json-append fields0 (validation-json-object-field "claim" (validation-source-id-json left)))
            (validation-json-append fields0 (validation-json-object-field "observation" (validation-source-id-json left))))))
    fields2
      (if (= relation (source-edge-motivates))
        (validation-json-append fields1 (validation-json-object-field "claim" (validation-source-id-json right)))
        (if (= relation (source-edge-constrained-by))
          (validation-json-append fields1 (validation-json-object-field "assumption" (validation-source-id-json right)))
          (if (= relation (source-edge-tested-by))
            (validation-json-append fields1 (validation-json-object-field "contract" (validation-source-id-json right)))
            (validation-json-append fields1 (validation-json-object-field "claim" (validation-source-id-json right))))))]
    (validation-json-object-wrap fields2)))

(defn validation-source-edges-json-state-loop [state]
  (let [edges (vector-get state 0)
    idx (vector-get state 1)
    len (vector-get state 2)
    out (vector-get state 3)]
    (if (>= idx len)
      out
      (let [next-out (validation-json-append out (validation-source-edge-json (vector-get edges idx)))
        next-indexed-state (vector-set-at-rooted-v3 state 1 (+ idx 1))
        next-state (vector-set-at-rooted-v3 next-indexed-state 3 next-out)]
        (validation-source-edges-json-state-loop next-state)))))

(defn validation-source-manifest-json [graph]
  (let [nodes (source-graph-nodes graph)
    edges (source-graph-edges graph)
    registry (source-evidence-graph-registry graph)
    nodes-state (validation-source-manifest-json-state
      (vector-push-single-rooted-v3
        (vector-push-single-rooted-v3
          (vector-push-single-rooted-v3
            (vector-push-single-rooted-v3 (vector-new 4) nodes)
            0)
          (vector-length nodes))
        ""))
    evidence-state (validation-source-manifest-json-state
      (vector-push-single-rooted-v3
        (vector-push-single-rooted-v3
          (vector-push-single-rooted-v3
            (vector-push-single-rooted-v3 (vector-new 4) registry)
            0)
          (vector-length registry))
        ""))
    edges-state (validation-source-manifest-json-state
      (vector-push-single-rooted-v3
        (vector-push-single-rooted-v3
          (vector-push-single-rooted-v3
            (vector-push-single-rooted-v3 (vector-new 4) edges)
            0)
          (vector-length edges))
        ""))
    nodes-json (validation-source-nodes-json-state-loop nodes-state)
    evidence-json (validation-source-evidence-json-state-loop evidence-state)
    edges-json (validation-source-edges-json-state-loop edges-state)
    fields0 (validation-json-int-field "schema_version" 1)
    fields1 (validation-json-append fields0
      (validation-json-array-field "nodes"
        (validation-json-array-wrap nodes-json)))
    fields2 (validation-json-append fields1
      (validation-json-array-field "evidence"
        (validation-json-array-wrap evidence-json)))
    fields3 (validation-json-append fields2
      (validation-json-array-field "edges"
        (validation-json-array-wrap edges-json)))]
    (validation-json-object-wrap fields3)))

(defn source-evidence-edge-form-result [form registry nodes]
  (if (< (vector-length form) 4)
    (source-result 0 (source-graph-error (source-error-malformed) 0 ""))
    (let [relation (vector-get form 0)
      payload (vector-get form 1)
      start (vector-get form 2)
      end (vector-get form 3)]
      (if (or (< (vector-length payload) 2)
          (and (!= relation (source-edge-supports)) (!= relation (source-edge-contradicts))))
        (source-edge-form-result form nodes)
        (let [evidence-id (vector-get payload 0)
          claim-id (vector-get payload 1)]
          (if (or (= (string-length evidence-id) 0) (= (string-length claim-id) 0))
            (source-result 0 (source-graph-error-at (source-error-malformed) relation evidence-id start end))
            (if (= (source-wire-valid? evidence-id (source-edge-supports)) 0)
              (source-result 0 (source-graph-error-at (source-error-invalid-id) relation evidence-id start end))
              (if (= (source-wire-valid? claim-id (source-node-claim)) 0)
                (source-result 0 (source-graph-error-at (source-error-invalid-id) relation claim-id start end))
                (if (= (source-node-id-exists? nodes claim-id) 0)
                  (source-result 0 (source-graph-error-at (source-error-missing-node) relation claim-id start end))
                  (if (= (source-evidence-id-exists? registry evidence-id) 0)
                    (source-result 0 (source-graph-error-at (source-error-evidence-registry-required) relation evidence-id start end))
                    (source-result 1 (source-edge-record relation evidence-id claim-id start end))))))))))))

(defn source-evidence-append-edge-forms [forms idx len registry nodes edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)]
      (if (source-edge-kind? kind)
        (let [parsed (source-evidence-edge-form-result form registry nodes)]
          (if (= (source-result-status parsed) 0)
            parsed
            (source-evidence-append-edge-forms
              forms
              (+ idx 1)
              len
              registry
              nodes
              (vector-push-single-rooted-v3 edges (source-result-value parsed)))))
        (source-evidence-append-edge-forms forms (+ idx 1) len registry nodes edges)))))

(defn source-evidence-collect-edge-children [decl idx len registry nodes edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [child (vector-get decl idx)
      parsed (source-evidence-collect-edges-decl child registry nodes edges)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-evidence-collect-edge-children
          decl
          (+ idx 1)
          len
          registry
          nodes
          (source-result-value parsed))))))

(defn source-evidence-collect-edges-decl [decl registry nodes edges]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (source-ordered-forms decl)]
        (source-evidence-append-edge-forms
          forms
          0
          (vector-length forms)
          registry
          nodes
          edges))
      (if (= tag (ast-private))
        (source-evidence-collect-edges-decl (vector-get decl 1) registry nodes edges)
        (if (= tag (ast-module-decl))
          (source-evidence-collect-edge-children decl 5 (vector-length decl) registry nodes edges)
          (if (= tag (ast-impldef))
            (source-evidence-collect-edge-children decl 4 (vector-length decl) registry nodes edges)
            (source-result 1 edges)))))))

(defn source-evidence-collect-edges-program-loop [program idx len registry nodes edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [parsed (source-evidence-collect-edges-decl
        (vector-get program idx)
        registry
        nodes
        edges)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-evidence-collect-edges-program-loop
          program
          (+ idx 1)
          len
          registry
          nodes
          (source-result-value parsed))))))

(defn source-evidence-collect-edges [program registry nodes]
  (source-evidence-collect-edges-program-loop
    program
    0
    (vector-length program)
    registry
    nodes
    (vector-new 0)))

(defn source-evidence-graph-from-program [program]
  (let [nodes-result (source-collect-nodes program)]
    (if (= (source-result-status nodes-result) 0)
      nodes-result
      (let [nodes (source-result-value nodes-result)
        registry-result (source-evidence-registry-from-program program)]
        (if (= (source-result-status registry-result) 0)
          registry-result
          (let [registry (source-result-value registry-result)
            edges-result (source-evidence-collect-edges program registry nodes)]
            (if (= (source-result-status edges-result) 0)
              edges-result
              (source-result
                1
                (source-evidence-graph
                  nodes
                  (source-result-value edges-result)
                  registry)))))))))
