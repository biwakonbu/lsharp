(module Tools.Validation.ManifestInput)

(import Tools.Validation.IntentSource)
(import Tools.Validation.ReviewIdentity)

;; Native source-file smoke の positional version 1 manifest input 境界。
;;
;; selfhost の JSON decoder がまだ typed manifest graph へ接続されていない間も、
;; source が emit した canonical manifest を同じ report/exit boundary へ戻せるよう、
;; manifest の固定 wire fields を fail-closed に確認する。generic JSON を黙って無視せず、
;; graph の trace を再構築できない形は呼び出し側で拒否する。

(defn manifest-input-ws? [ch]
  (if
    (or
      (or (= ch 32) (= ch 9))
      (or (= ch 10) (= ch 13)))
    1
    0))

(defn manifest-input-first-nonspace-loop [src idx len]
  (if (>= idx len)
    -1
    (if (= (manifest-input-ws? (string-char-at src idx)) 1)
      (manifest-input-first-nonspace-loop src (+ idx 1) len)
      idx)))

(defn manifest-input-pattern-at? [src pattern idx]
  (let [pattern-len (string-length pattern)
    end (+ idx pattern-len)]
    (if (> end (string-length src))
      0
      (if (string-eq (substring src idx end) pattern) 1 0))))

(defn manifest-input-find-pattern-loop [src pattern idx len]
  (if (>= idx len)
    -1
    (if (= (manifest-input-pattern-at? src pattern idx) 1)
      idx
      (manifest-input-find-pattern-loop src pattern (+ idx 1) len))))

(defn manifest-input-find-pattern [src pattern]
  (manifest-input-find-pattern-loop src pattern 0 (string-length src)))

(defn manifest-input-find-char-loop [src target idx len]
  (if (>= idx len)
    -1
    (if (= (string-char-at src idx) target)
      idx
      (manifest-input-find-char-loop src target (+ idx 1) len))))

(defn manifest-input-find-char [src target idx]
  (manifest-input-find-char-loop src target idx (string-length src)))

(defn validation-manifest-identity-field-result [src field optional start end]
  (let [string-marker
      (string-concat
        "\""
        (string-concat field "\":\""))
    string-start (manifest-input-find-pattern-loop src string-marker start end)
    null-marker
      (string-concat
        "\""
        (string-concat field "\":null"))
    null-start (manifest-input-find-pattern-loop src null-marker start end)]
    (if (>= string-start 0)
      (let [value-start (+ string-start (string-length string-marker))
        value-end (manifest-input-find-char-loop src 34 value-start end)]
        (if (and (> value-end value-start) (<= value-end end))
          (source-result 1 (substring src value-start value-end))
          (source-result 0 (source-review-evidence-identity-error field ""))))
      (if (and (= optional 1) (>= null-start 0))
        (source-result 1 "")
        (source-result 0 (source-review-evidence-identity-error field ""))))))

(defn validation-manifest-review-identity-result [src]
  (let [marker "\"review_evidence_identity\":"
    marker-start (manifest-input-find-pattern src marker)]
    (if (< marker-start 0)
      (source-result 1 (vector-new 0))
      (if (= (manifest-input-pattern-at? src "\"review_evidence_identity\":null" marker-start) 1)
        (source-result
          0
          (source-review-evidence-identity-error "review_evidence_identity" ""))
        (if (= (manifest-input-pattern-at? src "\"review_evidence_identity\":{" marker-start) 0)
          (source-result
            0
            (source-review-evidence-identity-error "review_evidence_identity" ""))
          (let [object-start (+ marker-start (string-length marker))
            object-end (manifest-input-find-char src 125 object-start)]
            (if (< object-end 0)
              (source-result
                0
                (source-review-evidence-identity-error "review_evidence_identity" ""))
              (let [subject-result
                  (validation-manifest-identity-field-result
                    src "subject_digest" 0 object-start object-end)
                source-commit-result
                  (validation-manifest-identity-field-result
                    src "source_commit" 0 object-start object-end)
                artifact-result
                  (validation-manifest-identity-field-result
                    src "artifact_digest" 0 object-start object-end)
                trust-result
                  (validation-manifest-identity-field-result
                    src "trust_store_digest" 1 object-start object-end)
                lifecycle-result
                  (validation-manifest-identity-field-result
                    src "lifecycle_digest" 1 object-start object-end)
                now-result
                  (validation-manifest-identity-field-result
                    src "now" 0 object-start object-end)]
                (if (= (source-result-status subject-result) 0)
                  subject-result
                  (if (= (source-result-status source-commit-result) 0)
                    source-commit-result
                    (if (= (source-result-status artifact-result) 0)
                      artifact-result
                      (if (= (source-result-status trust-result) 0)
                        trust-result
                        (if (= (source-result-status lifecycle-result) 0)
                          lifecycle-result
                          (if (= (source-result-status now-result) 0)
                            now-result
                            (source-review-evidence-identity-result
                              (source-result-value subject-result)
                              (source-result-value source-commit-result)
                              (source-result-value artifact-result)
                              (source-result-value trust-result)
                              (source-result-value lifecycle-result)
                              (source-result-value now-result))))))))))))))))

(defn manifest-input-count-pattern-loop [src pattern idx len count]
  (let [found (manifest-input-find-pattern-loop src pattern idx len)]
    (if (< found 0)
      count
      (manifest-input-count-pattern-loop
        src
        pattern
        (+ found (string-length pattern))
        len
        (+ count 1)))))

(defn manifest-input-count-pattern [src pattern]
  (manifest-input-count-pattern-loop src pattern 0 (string-length src) 0))

(defn validation-manifest-json-input? [src]
  (let [start (manifest-input-first-nonspace-loop src 0 (string-length src))]
      (if (and (>= start 0) (= (string-char-at src start) 123))
      (if (and
            (>= (manifest-input-find-pattern src "\"schema_version\":1") 0)
            (and
              (>= (manifest-input-find-pattern src "\"nodes\":[") 0)
              (and
                (>= (manifest-input-find-pattern src "\"evidence\":[") 0)
                (>= (manifest-input-find-pattern src "\"edges\":[") 0))))
        1
        0)
      0)))

(defn validation-manifest-positional-path? [value]
  (if (and
        (> (string-length value) 0)
        (!= (string-char-at value 0) 45))
    1
    0))

(defn validation-manifest-required-trace? [src]
  (if (and
        (>= (manifest-input-find-pattern src "\"relation\":\"motivates\"") 0)
        (>= (manifest-input-find-pattern src "\"relation\":\"tested-by\"") 0))
    1
    0))

(defn validation-manifest-open-question-count [src]
  (manifest-input-count-pattern src "\"kind\":\"open-question\""))

(defn validation-manifest-independent-review-count [src]
  (let [method-count (manifest-input-count-pattern src "\"method\":\"review\"")
    independence-count
      (manifest-input-count-pattern src "\"independence\":\"independent-review\"")]
    (if (< method-count independence-count) method-count independence-count)))

(defn validation-manifest-contradiction-count [src]
  (+
    (manifest-input-count-pattern src "\"outcome\":\"contradicted\"")
    (manifest-input-count-pattern src "\"relation\":\"contradicts\"")))

(defn validation-manifest-stale-review-count [src]
  (manifest-input-count-pattern src "\"relation\":\"invalidates\""))

(defn validation-manifest-stale-evidence-count [src]
  (manifest-input-count-pattern src "\"outcome\":\"stale\""))
