(module Tools.Validation.ReviewIdentity)
(import Tools.Validation.IntentSource)
(import Tools.Validation.Whitespace)

;; caller が明示した review evidence identity。
;; identity: [subject-digest, source-commit, artifact-digest,
;;            trust-store-digest-or-empty, lifecycle-digest-or-empty, now]

(defn source-review-evidence-identity-error-code [] 14)
(defn source-review-evidence-identity [subject-digest source-commit artifact-digest trust-store-digest lifecycle-digest now]
  (let [base (vector-push-quad-rooted-v3
      (vector-new 1)
      subject-digest
      source-commit
      artifact-digest
      trust-store-digest)
    with-lifecycle (vector-push-single-rooted-v3 base lifecycle-digest)]
    (vector-push-single-rooted-v3 with-lifecycle now)))
(defn source-review-evidence-identity-subject-digest [identity] (vector-get identity 0))
(defn source-review-evidence-identity-source-commit [identity] (vector-get identity 1))
(defn source-review-evidence-identity-artifact-digest [identity] (vector-get identity 2))
(defn source-review-evidence-identity-trust-store-digest [identity] (vector-get identity 3))
(defn source-review-evidence-identity-lifecycle-digest [identity] (vector-get identity 4))
(defn source-review-evidence-identity-now [identity] (vector-get identity 5))
(defn source-review-evidence-identity-required-valid? [value]
  (validation-nonblank? value))
(defn source-review-evidence-identity-optional-valid? [value]
  (if (= (string-length value) 0)
    1
    (validation-nonblank? value)))
(defn source-review-evidence-identity-error [field value]
  (let [base (vector-push-quad-rooted-v3
      (vector-new 1)
      (source-review-evidence-identity-error-code)
      field
      value
      -1)
    with-end (vector-push-single-rooted-v3 base -1)
    with-related-start (vector-push-single-rooted-v3 with-end -1)]
    (vector-push-single-rooted-v3 with-related-start -1)))
(defn source-review-evidence-identity-result
  [subject-digest source-commit artifact-digest trust-store-digest lifecycle-digest now]
  (if (= (source-review-evidence-identity-required-valid? subject-digest) 0)
    (source-result 0 (source-review-evidence-identity-error "subject_digest" subject-digest))
    (if (= (source-review-evidence-identity-required-valid? source-commit) 0)
      (source-result 0 (source-review-evidence-identity-error "source_commit" source-commit))
      (if (= (source-review-evidence-identity-required-valid? artifact-digest) 0)
        (source-result 0 (source-review-evidence-identity-error "artifact_digest" artifact-digest))
        (if (= (source-review-evidence-identity-optional-valid? trust-store-digest) 0)
          (source-result 0 (source-review-evidence-identity-error "trust_store_digest" trust-store-digest))
          (if (= (source-review-evidence-identity-optional-valid? lifecycle-digest) 0)
            (source-result 0 (source-review-evidence-identity-error "lifecycle_digest" lifecycle-digest))
            (if (= (source-review-evidence-identity-required-valid? now) 0)
              (source-result 0 (source-review-evidence-identity-error "now" now))
              (if (= (source-review-attestation-timestamp-valid? now) 0)
                (source-result 0 (source-review-evidence-identity-error "now" now))
                (source-result
                  1
                  (source-review-evidence-identity
                    subject-digest
                    source-commit
                    artifact-digest
                    trust-store-digest
                    lifecycle-digest
                    now))))))))))
(defn source-review-evidence-identity-equal? [first second]
  (and
    (string-eq
      (source-review-evidence-identity-subject-digest first)
      (source-review-evidence-identity-subject-digest second))
    (and
      (string-eq
        (source-review-evidence-identity-source-commit first)
        (source-review-evidence-identity-source-commit second))
      (and
        (string-eq
          (source-review-evidence-identity-artifact-digest first)
          (source-review-evidence-identity-artifact-digest second))
        (and
          (string-eq
            (source-review-evidence-identity-trust-store-digest first)
            (source-review-evidence-identity-trust-store-digest second))
          (and
            (string-eq
              (source-review-evidence-identity-lifecycle-digest first)
              (source-review-evidence-identity-lifecycle-digest second))
            (string-eq
              (source-review-evidence-identity-now first)
              (source-review-evidence-identity-now second))))))))
(defn source-review-evidence-identity-value-valid? [identity]
  (if (!= (vector-length identity) 6)
    0
    (source-result-status
      (source-review-evidence-identity-result
        (source-review-evidence-identity-subject-digest identity)
        (source-review-evidence-identity-source-commit identity)
        (source-review-evidence-identity-artifact-digest identity)
        (source-review-evidence-identity-trust-store-digest identity)
        (source-review-evidence-identity-lifecycle-digest identity)
        (source-review-evidence-identity-now identity)))))
(defn source-evidence-graph-review-identity [graph]
  (if (> (vector-length graph) 5)
    (vector-get graph 5)
    0))
(defn source-evidence-graph-attach-review-identity [graph identity]
  (if (= (source-review-evidence-identity-value-valid? identity) 0)
    (source-result 0 (source-review-evidence-identity-error "review_evidence_identity" ""))
    (if (> (vector-length graph) 5)
      (if (source-review-evidence-identity-equal?
            (source-evidence-graph-review-identity graph)
            identity)
        (source-result 1 graph)
        (source-result
          0
          (source-review-evidence-identity-error "review_evidence_identity" "")))
      (source-result 1 (vector-push-single-rooted-v3 graph identity)))))

(defn source-review-identity-json-append [out piece]
  (if (= (string-length out) 0) piece (string-concat out (string-concat "," piece))))
(defn source-review-identity-json-field [name value-json]
  (string-concat "\"" (string-concat name (string-concat "\":" value-json))))
(defn source-review-identity-json-string-literal [value]
  (string-concat "\"" (string-concat (json-escape-string value) "\"")))
(defn source-review-identity-json-string-field [name value]
  (source-review-identity-json-field name (source-review-identity-json-string-literal value)))
(defn source-review-identity-json-null-field [name]
  (source-review-identity-json-field name "null"))
(defn source-review-evidence-identity-json [identity]
  (let [fields0 (source-review-identity-json-string-field
      "subject_digest"
      (source-review-evidence-identity-subject-digest identity))
    fields1 (source-review-identity-json-append fields0
      (source-review-identity-json-string-field
        "source_commit"
        (source-review-evidence-identity-source-commit identity)))
    fields2 (source-review-identity-json-append fields1
      (source-review-identity-json-string-field
        "artifact_digest"
        (source-review-evidence-identity-artifact-digest identity)))
    trust-store-digest (source-review-evidence-identity-trust-store-digest identity)
    fields3 (if (= (string-length trust-store-digest) 0)
      (source-review-identity-json-append fields2
        (source-review-identity-json-null-field "trust_store_digest"))
      (source-review-identity-json-append fields2
        (source-review-identity-json-string-field "trust_store_digest" trust-store-digest)))
    lifecycle-digest (source-review-evidence-identity-lifecycle-digest identity)
    fields4 (if (= (string-length lifecycle-digest) 0)
      (source-review-identity-json-append fields3
        (source-review-identity-json-null-field "lifecycle_digest"))
      (source-review-identity-json-append fields3
        (source-review-identity-json-string-field "lifecycle_digest" lifecycle-digest)))
    fields5 (source-review-identity-json-append fields4
      (source-review-identity-json-string-field
        "now"
        (source-review-evidence-identity-now identity)))]
    (string-concat "{" (string-concat fields5 "}"))))
