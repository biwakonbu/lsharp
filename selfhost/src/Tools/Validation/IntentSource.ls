(module Tools.Validation.IntentSource)
(import Syntax.AST)
(import Syntax.Parser)
(import Tools.Validation.Whitespace)

;; M2 source metadata を selfhost の tagged vector graph へ投影する。
;;
;; record の wire shape は Rust の source adapter と同じ意味を持つ。
;; node: [kind, stable-id, text, span-start, span-end]
;; edge: [relation, left-id, right-id, span-start, span-end]
;; graph: [nodes, edges] または [nodes, edges, reviews]
;; review: [stable-id, provenance-digest, visibility, span-start, span-end]
;; result: [status, graph-or-error] (1=success, 0=fail)
;; error: [code, form-kind, offending-id, span-start, span-end,
;;         related-span-start, related-span-end]

(defn source-node-intent [] 6)
(defn source-node-claim [] 7)
(defn source-node-assumption [] 8)
(defn source-node-open-question [] 9)
(defn source-edge-motivates [] 10)
(defn source-edge-constrained-by [] 11)
(defn source-edge-tested-by [] 12)
(defn source-edge-supports [] 13)
(defn source-edge-contradicts [] 14)
(defn source-review [] 16)
(defn source-edge-evaluates [] 17)
(defn source-edge-invalidates [] 18)
(defn source-change [] 19)

(defn source-error-malformed [] 1)
(defn source-error-invalid-id [] 2)
(defn source-error-kind-mismatch [] 3)
(defn source-error-duplicate-node [] 4)
(defn source-error-missing-node [] 5)
(defn source-error-evidence-registry-required [] 6)
(defn source-error-duplicate-review [] 7)
(defn source-error-invalid-review [] 8)
(defn source-error-edge-subject-kind-mismatch [] 9)
(defn source-error-missing-review [] 10)
(defn source-error-review-edge-consumer-required [] 11)

(defn source-result [status value]
  (vector-push-pair-rooted-v3 (vector-new 2) status value))

(defn source-result-status [result] (vector-get result 0))
(defn source-result-value [result] (vector-get result 1))
(defn source-result-error [result] (vector-get result 1))
;; public graph-result aliases keep the adapter surface explicit at call sites.
(defn source-graph-result-status [result] (source-result-status result))
(defn source-graph-result-value [result] (source-result-value result))
(defn source-graph-result-error [result] (source-result-error result))

(defn source-graph [nodes edges]
  (vector-push-pair-rooted-v3 (vector-new 2) nodes edges))

(defn source-graph-with-reviews [nodes edges reviews]
  (vector-push-triple-rooted-v3 (vector-new 3) nodes edges reviews))

(defn source-graph-nodes [graph] (vector-get graph 0))
(defn source-graph-edges [graph] (vector-get graph 1))
(defn source-graph-reviews [graph]
  (if (> (vector-length graph) 2)
    (vector-get graph 2)
    (vector-new 0)))

(defn source-graph-error-record [code kind id start end related-start related-end]
  (let [base (vector-push-quad-rooted-v3 (vector-new 1) code kind id start)
    with-end (vector-push-single-rooted-v3 base end)
    with-related-start (vector-push-single-rooted-v3 with-end related-start)]
    (vector-push-single-rooted-v3 with-related-start related-end)))

(defn source-graph-error [code kind id]
  (source-graph-error-record code kind id -1 -1 -1 -1))

(defn source-graph-error-at [code kind id start end]
  (source-graph-error-record code kind id start end -1 -1))

(defn source-graph-error-related [code kind id start end related-start related-end]
  (source-graph-error-record code kind id start end related-start related-end))

;; malformed tagged form でも、存在する kind/span は診断へ引き継ぐ。
;; 欠落したフィールドの offset は未取得を示す -1 とする。
(defn source-form-kind-or-zero [form]
  (if (> (vector-length form) 0)
    (vector-get form 0)
    0))

(defn source-form-start-or-minus-one [form]
  (if (> (vector-length form) 2)
    (vector-get form 2)
    -1))

(defn source-form-end-or-minus-one [form]
  (if (> (vector-length form) 3)
    (vector-get form 3)
    -1))

(defn source-form-malformed-error [form]
  (source-graph-error-record
    (source-error-malformed)
    (source-form-kind-or-zero form)
    ""
    (source-form-start-or-minus-one form)
    (source-form-end-or-minus-one form)
    -1
    -1))

(defn source-graph-error-code [error] (vector-get error 0))
(defn source-graph-error-kind [error] (vector-get error 1))
(defn source-graph-error-id [error] (vector-get error 2))
(defn source-graph-error-start [error] (vector-get error 3))
(defn source-graph-error-end [error] (vector-get error 4))
(defn source-graph-error-related-start [error] (vector-get error 5))
(defn source-graph-error-related-end [error] (vector-get error 6))

(defn source-node-record [kind id text start end]
  (let [base (vector-push-quad-rooted-v3 (vector-new 1) kind id text start)]
    (vector-push-single-rooted-v3 base end)))

(defn source-node-kind [node] (vector-get node 0))
(defn source-node-id [node] (vector-get node 1))
(defn source-node-text [node] (vector-get node 2))
(defn source-node-start [node] (vector-get node 3))
(defn source-node-end [node] (vector-get node 4))

(defn source-edge-record [relation left right start end]
  (let [base (vector-push-quad-rooted-v3 (vector-new 1) relation left right start)]
    (vector-push-single-rooted-v3 base end)))

(defn source-edge-kind [edge] (vector-get edge 0))
(defn source-edge-left [edge] (vector-get edge 1))
(defn source-edge-right [edge] (vector-get edge 2))
(defn source-edge-start [edge] (vector-get edge 3))
(defn source-edge-end [edge] (vector-get edge 4))

(defn source-review-record [id provenance-digest visibility start end]
  (let [base (vector-push-quad-rooted-v3
      (vector-new 1)
      id
      provenance-digest
      visibility
      start)]
    (vector-push-single-rooted-v3 base end)))

(defn source-review-id [review] (vector-get review 0))
(defn source-review-provenance-digest [review] (vector-get review 1))
(defn source-review-visibility [review] (vector-get review 2))
(defn source-review-start [review] (vector-get review 3))
(defn source-review-end [review] (vector-get review 4))

(defn source-node-kind? [kind]
  (and (>= kind (source-node-intent)) (<= kind (source-node-open-question))))

(defn source-edge-kind? [kind]
  (or
    (and (>= kind (source-edge-motivates)) (<= kind (source-edge-contradicts)))
    (or (= kind (source-edge-evaluates)) (= kind (source-edge-invalidates)))))

(defn source-kind-prefix [kind]
  (if (= kind (source-node-intent)) "intent"
    (if (= kind (source-node-claim)) "claim"
      (if (= kind (source-node-assumption)) "assumption"
        (if (= kind (source-node-open-question)) "open-question"
          (if (= kind (source-edge-tested-by)) "contract"
              (if (or (= kind (source-edge-supports)) (= kind (source-edge-contradicts)))
              "evidence"
              (if (= kind (source-review)) "review"
                (if (= kind (source-change)) "change" "")))))))))

(defn source-find-char [text target idx len]
  (if (>= idx len)
    -1
    (if (= (string-char-at text idx) target)
      idx
      (source-find-char text target (+ idx 1) len))))

(defn source-id-segment-char? [char]
  (or
    (or
      (or
        (and (>= char 48) (<= char 57))
        (and (>= char 65) (<= char 90)))
      (and (>= char 97) (<= char 122)))
    (or (or (= char 95) (= char 45)) (= char 46))))

(defn source-id-segment-valid-loop [wire idx end]
  (if (>= idx end)
    1
    (if (source-id-segment-char? (string-char-at wire idx))
      (source-id-segment-valid-loop wire (+ idx 1) end)
      0)))

;; stable ID の wire format を node/edge の期待 kind と同時に検証する。
(defn source-wire-valid? [wire expected-kind]
  (let [len (string-length wire)
    colon (source-find-char wire 58 0 len)
    slash (if (>= colon 0) (source-find-char wire 47 (+ colon 1) len) -1)
    prefix (if (> colon 0) (substring wire 0 colon) "")]
    (if (or (or (<= colon 0) (<= slash (+ colon 1))) (>= (+ slash 1) len))
      0
      (if (string-eq prefix (source-kind-prefix expected-kind))
        (if (and
              (= (source-id-segment-valid-loop wire (+ colon 1) slash) 1)
              (= (source-id-segment-valid-loop wire (+ slash 1) len) 1))
          1
          0)
        0))))

;; prefix の意味は後段で判定する edge subject 向けに、wire の構文だけを検証する。
(defn source-wire-shape-valid? [wire]
  (let [len (string-length wire)
    colon (source-find-char wire 58 0 len)
    slash (if (>= colon 0) (source-find-char wire 47 (+ colon 1) len) -1)]
    (if (or (or (<= colon 0) (<= slash (+ colon 1))) (>= (+ slash 1) len))
      0
      (if (and
            (= (source-id-segment-valid-loop wire (+ colon 1) slash) 1)
            (= (source-id-segment-valid-loop wire (+ slash 1) len) 1))
        1
        0))))

(defn source-review-subject-kind [wire]
  (if (= (source-wire-valid? wire (source-node-intent)) 1)
    (source-node-intent)
    (if (= (source-wire-valid? wire (source-node-claim)) 1)
      (source-node-claim)
      (if (= (source-wire-valid? wire (source-edge-supports)) 1)
        (source-edge-supports)
        0))))

(defn source-invalidation-subject-kind [wire]
  (if (= (source-wire-valid? wire (source-review)) 1)
    (source-review)
    (if (= (source-wire-valid? wire (source-edge-supports)) 1)
      (source-edge-supports)
      0)))

(defn source-wire-valid-node? [wire]
  (or
    (or
      (= (source-wire-valid? wire (source-node-intent)) 1)
      (= (source-wire-valid? wire (source-node-claim)) 1))
    (or
      (= (source-wire-valid? wire (source-node-assumption)) 1)
      (= (source-wire-valid? wire (source-node-open-question)) 1))))

(defn source-node-id-exists-loop [nodes id idx len]
  (if (>= idx len)
    0
    (if (string-eq (source-node-id (vector-get nodes idx)) id)
      1
      (source-node-id-exists-loop nodes id (+ idx 1) len))))

(defn source-node-id-exists? [nodes id]
  (source-node-id-exists-loop nodes id 0 (vector-length nodes)))

(defn source-node-find-loop [nodes id idx len]
  (if (>= idx len)
    0
    (let [node (vector-get nodes idx)]
      (if (string-eq (source-node-id node) id)
        node
        (source-node-find-loop nodes id (+ idx 1) len)))))

(defn source-node-find [nodes id]
  (source-node-find-loop nodes id 0 (vector-length nodes)))

(defn source-defn-metadata [decl]
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

(defn source-ordered-forms [decl]
  (let [metadata (source-defn-metadata decl)]
    (if (= metadata 0)
      (vector-new 0)
      (if (> (vector-length metadata) 5)
        (vector-get metadata 5)
        (vector-new 0)))))

;; type / record 宣言は metadata を payload の末尾へ保持する。
;; metadata がない短い宣言 ([tag, name]) は整数の name を誤って読むため除外する。
(defn source-type-metadata [decl]
  (let [decl-length (vector-length decl)]
    (if (> decl-length 2)
      (let [candidate (vector-get decl (- decl-length 1))]
        (if (> (vector-length candidate) 5)
          candidate
          0))
      0)))

(defn source-type-ordered-forms [decl]
  (let [metadata (source-type-metadata decl)]
    (if (= metadata 0)
      (vector-new 0)
      (vector-get metadata 5))))

(defn source-node-nonblank? [value]
  (validation-nonblank? value))

(defn source-node-form-result [form]
  (if (< (vector-length form) 4)
    (source-result 0 (source-form-malformed-error form))
    (let [kind (vector-get form 0)
      payload (vector-get form 1)
      start (vector-get form 2)
      end (vector-get form 3)]
      (if (!= (vector-length form) 4)
        (source-result 0 (source-graph-error-at (source-error-malformed) kind "" start end))
        (if (!= (vector-length payload) 2)
          (source-result 0 (source-graph-error-at (source-error-malformed) kind "" start end))
          (let [id (vector-get payload 0)
            text (vector-get payload 1)]
            (if (or
                  (= (string-length id) 0)
                  (= (source-node-nonblank? text) 0))
              (source-result 0 (source-graph-error-at (source-error-malformed) kind id start end))
              (if (= (source-wire-valid? id kind) 0)
                (if (source-wire-valid-node? id)
                  (source-result 0 (source-graph-error-at (source-error-kind-mismatch) kind id start end))
                  (source-result 0 (source-graph-error-at (source-error-invalid-id) kind id start end)))
                (source-result 1 (source-node-record kind id text start end))))))))))

(defn source-review-id-exists-loop [reviews id idx len]
  (if (>= idx len)
    0
    (if (string-eq (source-review-id (vector-get reviews idx)) id)
      1
      (source-review-id-exists-loop reviews id (+ idx 1) len))))

(defn source-review-id-exists? [reviews id]
  (source-review-id-exists-loop reviews id 0 (vector-length reviews)))

(defn source-string-id-exists-loop [ids id idx len]
  (if (>= idx len) 0 (if (string-eq (vector-get ids idx) id) 1
    (source-string-id-exists-loop ids id (+ idx 1) len))))

(defn source-string-id-exists? [ids id] (source-string-id-exists-loop ids id 0 (vector-length ids)))
(defn source-review-find-loop [reviews id idx len]
  (if (>= idx len)
    0
    (let [review (vector-get reviews idx)]
      (if (string-eq (source-review-id review) id)
        review
        (source-review-find-loop reviews id (+ idx 1) len)))))

(defn source-review-find [reviews id]
  (source-review-find-loop reviews id 0 (vector-length reviews)))

(defn source-review-visibility-valid? [visibility]
  (or (string-eq visibility "public") (string-eq visibility "redacted")))

(defn source-review-nonblank? [value]
  (validation-nonblank? value))

(defn source-review-form-result [form]
  (if (< (vector-length form) 4)
    (source-result 0 (source-form-malformed-error form))
    (let [kind (vector-get form 0)
      payload (vector-get form 1)
      start (vector-get form 2)
      end (vector-get form 3)]
      (if (!= (vector-length form) 4)
        (source-result 0 (source-graph-error-at (source-error-malformed) kind "" start end))
        (if (!= (vector-length payload) 3)
          (source-result 0 (source-graph-error-at (source-error-malformed) kind "" start end))
          (let [id (vector-get payload 0)
            provenance-digest (vector-get payload 1)
            visibility (vector-get payload 2)]
            (if (or
                  (= (string-length id) 0)
                  (= (source-review-nonblank? provenance-digest) 0))
              (source-result 0 (source-graph-error-at (source-error-invalid-review) kind id start end))
              (if (= (source-wire-valid? id kind) 0)
                (source-result 0 (source-graph-error-at (source-error-invalid-id) kind id start end))
                (if (source-review-visibility-valid? visibility)
                  (source-result 1
                    (source-review-record id provenance-digest visibility start end))
                  (source-result 0
                    (source-graph-error-at (source-error-invalid-review) kind id start end)))))))))))

(defn source-append-review-forms [forms idx len reviews]
  (if (>= idx len)
    (source-result 1 reviews)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)]
      (if (= kind (source-review))
        (let [parsed (source-review-form-result form)]
          (if (= (source-result-status parsed) 0)
            parsed
            (let [review (source-result-value parsed)
              id (source-review-id review)
              existing (source-review-find reviews id)]
              (if (= (source-review-id-exists? reviews id) 1)
                (source-result 0
                  (source-graph-error-related
                    (source-error-duplicate-review)
                    kind
                    id
                    (source-review-start review)
                    (source-review-end review)
                    (source-review-start existing)
                    (source-review-end existing)))
                (let [next-reviews (vector-push-single-rooted-v3 reviews review)]
                  (source-append-review-forms forms (+ idx 1) len next-reviews))))))
        (source-append-review-forms forms (+ idx 1) len reviews)))))

(defn source-append-node-forms [forms idx len nodes]
  (if (>= idx len)
    (source-result 1 nodes)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)]
      (if (source-node-kind? kind)
        (let [parsed (source-node-form-result form)]
          (if (= (source-result-status parsed) 0)
            parsed
            (let [node (source-result-value parsed)
              id (source-node-id node)
              existing (source-node-find nodes id)]
              (if (= (source-node-id-exists? nodes id) 1)
                (source-result 0
                  (source-graph-error-related
                    (source-error-duplicate-node)
                    kind
                    id
                    (source-node-start node)
                    (source-node-end node)
                    (source-node-start existing)
                    (source-node-end existing)))
                (let [next-nodes (vector-push-single-rooted-v3 nodes node)]
                  (source-append-node-forms forms (+ idx 1) len next-nodes))))))
        (source-append-node-forms forms (+ idx 1) len nodes)))))

(defn source-collect-node-children [decl idx len nodes]
  (if (>= idx len)
    (source-result 1 nodes)
    (let [child (vector-get decl idx)
      parsed (source-collect-nodes-decl child nodes)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-node-children decl (+ idx 1) len (source-result-value parsed))))))

(defn source-collect-nodes-decl [decl nodes]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (source-ordered-forms decl)]
        (source-append-node-forms forms 0 (vector-length forms) nodes))
      (if (= tag (ast-private))
        (source-collect-nodes-decl (vector-get decl 1) nodes)
        (if (= tag (ast-module-decl))
          ;; module: [tag, name, count, name-start, name-end, child...]
          (source-collect-node-children decl 5 (vector-length decl) nodes)
          (if (= tag (ast-impldef))
            ;; impl: [tag, trait, type, count, child...]
            (source-collect-node-children decl 4 (vector-length decl) nodes)
            (if (or (= tag (ast-typedef)) (= tag (ast-recorddef)))
              (let [forms (source-type-ordered-forms decl)]
                (source-append-node-forms forms 0 (vector-length forms) nodes))
              (source-result 1 nodes))))))))

(defn source-collect-nodes-program-loop [program idx len nodes]
  (if (>= idx len)
    (source-result 1 nodes)
    (let [parsed (source-collect-nodes-decl (vector-get program idx) nodes)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-nodes-program-loop
          program
          (+ idx 1)
          len
          (source-result-value parsed))))))

(defn source-collect-nodes [program]
  (source-collect-nodes-program-loop program 0 (vector-length program) (vector-new 0)))

(defn source-collect-review-children [decl idx len reviews]
  (if (>= idx len)
    (source-result 1 reviews)
    (let [child (vector-get decl idx)
      parsed (source-collect-reviews-decl child reviews)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-review-children
          decl
          (+ idx 1)
          len
          (source-result-value parsed))))))

(defn source-collect-reviews-decl [decl reviews]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (source-ordered-forms decl)]
        (source-append-review-forms forms 0 (vector-length forms) reviews))
      (if (= tag (ast-private))
        (source-collect-reviews-decl (vector-get decl 1) reviews)
        (if (= tag (ast-module-decl))
          (source-collect-review-children decl 5 (vector-length decl) reviews)
          (if (= tag (ast-impldef))
            (source-collect-review-children decl 4 (vector-length decl) reviews)
            (if (or (= tag (ast-typedef)) (= tag (ast-recorddef)))
              (let [forms (source-type-ordered-forms decl)]
                (source-append-review-forms forms 0 (vector-length forms) reviews))
              (source-result 1 reviews))))))))

(defn source-collect-reviews-program-loop [program idx len reviews]
  (if (>= idx len)
    (source-result 1 reviews)
    (let [parsed (source-collect-reviews-decl (vector-get program idx) reviews)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-reviews-program-loop
          program
          (+ idx 1)
          len
          (source-result-value parsed))))))

(defn source-collect-reviews [program]
  (source-collect-reviews-program-loop program 0 (vector-length program) (vector-new 0)))

(defn source-edge-endpoint-kind [relation side]
  (if (= relation (source-edge-motivates))
    (if (= side 0) (source-node-intent) (source-node-claim))
    (if (= relation (source-edge-constrained-by))
      (if (= side 0) (source-node-claim) (source-node-assumption))
      (if (= relation (source-edge-tested-by))
        (if (= side 0) (source-node-claim) (source-edge-tested-by))
        (source-edge-supports)))))

(defn source-edge-form-result [form nodes]
  (if (< (vector-length form) 4)
    (source-result 0 (source-form-malformed-error form))
    (let [relation (vector-get form 0)
      payload (vector-get form 1)
      start (vector-get form 2)
      end (vector-get form 3)]
      (if (!= (vector-length form) 4)
        (source-result 0 (source-graph-error-at (source-error-malformed) relation "" start end))
        (if (!= (vector-length payload) 2)
          (source-result 0 (source-graph-error-at (source-error-malformed) relation "" start end))
          (let [left (vector-get payload 0)
            right (vector-get payload 1)
            left-kind (source-edge-endpoint-kind relation 0)
            right-kind (source-edge-endpoint-kind relation 1)]
            (if (or (= (string-length left) 0) (= (string-length right) 0))
              (source-result 0 (source-graph-error-at (source-error-malformed) relation left start end))
              (if (or (= relation (source-edge-supports)) (= relation (source-edge-contradicts)))
                (source-result 0
                  (source-graph-error-at
                    (source-error-evidence-registry-required)
                    relation
                    left
                    start
                    end))
                (if (= (source-wire-valid? left left-kind) 0)
                  (source-result 0 (source-graph-error-at (source-error-invalid-id) relation left start end))
                  (if (= (source-wire-valid? right right-kind) 0)
                    (source-result 0 (source-graph-error-at (source-error-invalid-id) relation right start end))
                    (if (= (source-node-id-exists? nodes left) 0)
                      (source-result 0 (source-graph-error-at (source-error-missing-node) relation left start end))
                      (if (and
                            (!= right-kind (source-edge-tested-by))
                            (= (source-node-id-exists? nodes right) 0))
                        (source-result 0 (source-graph-error-at (source-error-missing-node) relation right start end))
                        (source-result 1 (source-edge-record relation left right start end))))))))))))))

(defn source-review-edge-form-result [form nodes reviews]
  (if (< (vector-length form) 4)
    (source-result 0 (source-form-malformed-error form))
    (let [relation (vector-get form 0)
      payload (vector-get form 1)
      start (vector-get form 2)
      end (vector-get form 3)]
      (if (or
            (!= (vector-length form) 4)
            (or
              (!= (vector-length payload) 2)
              (and (!= relation (source-edge-evaluates)) (!= relation (source-edge-invalidates)))))
        (source-result 0 (source-graph-error-at (source-error-malformed) relation "" start end))
        (let [left (vector-get payload 0)
          right (vector-get payload 1)]
          (if (or (= (string-length left) 0) (= (string-length right) 0))
            (source-result 0 (source-graph-error-at (source-error-malformed) relation left start end))
            (if (= relation (source-edge-evaluates))
              (if (= (source-wire-shape-valid? left) 0)
                (source-result 0 (source-graph-error-at (source-error-invalid-id) relation left start end))
                (if (= (source-wire-valid? left (source-review)) 0)
                  (source-result 0 (source-graph-error-at (source-error-kind-mismatch) relation left start end))
                  (if (and
                        (> (vector-length reviews) 0)
                        (= (source-review-id-exists? reviews left) 0))
                    (source-result 0 (source-graph-error-at (source-error-missing-review) relation left start end))
                    (if (= (source-wire-shape-valid? right) 0)
                      (source-result 0 (source-graph-error-at (source-error-invalid-id) relation right start end))
                      (let [subject-kind (source-review-subject-kind right)]
                        (if (= subject-kind 0)
                          (source-result 0
                            (source-graph-error-at
                              (source-error-edge-subject-kind-mismatch)
                              relation
                              right
                              start
                              end))
                          (if (= subject-kind (source-edge-supports))
                            (source-result 0
                              (source-graph-error-at
                                (source-error-evidence-registry-required)
                                relation
                                right
                                start
                                end))
                            (if (= (source-node-id-exists? nodes right) 0)
                              (source-result 0 (source-graph-error-at (source-error-missing-node) relation right start end))
                              (source-result 1 (source-edge-record relation left right start end))))))))))
              (if (= (source-wire-shape-valid? left) 0)
                (source-result 0 (source-graph-error-at (source-error-invalid-id) relation left start end))
                (if (= (source-wire-valid? left (source-change)) 0)
                  (source-result 0 (source-graph-error-at (source-error-kind-mismatch) relation left start end))
                  (if (= (source-wire-shape-valid? right) 0)
                    (source-result 0 (source-graph-error-at (source-error-invalid-id) relation right start end))
                    (let [subject-kind (source-invalidation-subject-kind right)]
                      (if (= subject-kind 0)
                        (source-result 0
                          (source-graph-error-at
                            (source-error-edge-subject-kind-mismatch)
                            relation
                            right
                            start
                            end))
                        (if (= subject-kind (source-edge-supports))
                          (source-result 0
                            (source-graph-error-at
                              (source-error-evidence-registry-required)
                              relation
                              right
                              start
                              end))
                          (if (and
                                (> (vector-length reviews) 0)
                                (= (source-review-id-exists? reviews right) 0))
                            (source-result 0 (source-graph-error-at (source-error-missing-review) relation right start end))
                            (source-result 1 (source-edge-record relation left right start end))))))))))))))))

(defn source-review-edge-form-result-with-evidence [form nodes reviews evidence-ids]
  (let [payload (if (> (vector-length form) 1) (vector-get form 1) (vector-new 0)) relation (if (> (vector-length form) 0) (vector-get form 0) 0) left (if (> (vector-length payload) 0) (vector-get payload 0) "") right (if (> (vector-length payload) 1) (vector-get payload 1) "") parsed (source-review-edge-form-result form nodes reviews)]
    (if (= (source-result-status parsed) 1) parsed
      (let [error (source-result-error parsed)]
        (if (and (= (source-graph-error-code error) (source-error-evidence-registry-required)) (= (source-string-id-exists? evidence-ids right) 1))
          (source-result 1 (source-edge-record relation left right (source-graph-error-start error) (source-graph-error-end error)))
          parsed)))))
(defn source-edge-form-result-with-reviews-and-evidence [form nodes reviews evidence-ids]
  (let [relation (if (> (vector-length form) 0) (vector-get form 0) 0)]
    (if (or (= relation (source-edge-evaluates)) (= relation (source-edge-invalidates)))
      (source-review-edge-form-result-with-evidence form nodes reviews evidence-ids)
      (source-edge-form-result form nodes))))
(defn source-append-edge-forms-with-reviews [forms idx len nodes reviews edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [form (vector-get forms idx) kind (vector-get form 0)]
      (if (source-edge-kind? kind)
        (let [parsed (source-edge-form-result-with-reviews-and-evidence form nodes reviews (vector-new 0))]
          (if (= (source-result-status parsed) 0)
            parsed
            (source-append-edge-forms-with-reviews forms (+ idx 1) len nodes reviews
              (vector-push-single-rooted-v3 edges (source-result-value parsed)))))
        (source-append-edge-forms-with-reviews forms (+ idx 1) len nodes reviews edges)))))

(defn source-append-edge-forms [forms idx len nodes edges]
  (source-append-edge-forms-with-reviews forms idx len nodes (vector-new 0) edges))

(defn source-collect-edge-children-with-reviews [decl idx len nodes reviews edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [child (vector-get decl idx)
      parsed (source-collect-edges-decl-with-reviews child nodes reviews edges)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-edge-children-with-reviews decl (+ idx 1) len nodes reviews
          (source-result-value parsed))))))

(defn source-collect-edges-decl-with-reviews [decl nodes reviews edges]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (source-ordered-forms decl)]
        (source-append-edge-forms-with-reviews
          forms
          0
          (vector-length forms)
          nodes
          reviews
          edges))
      (if (= tag (ast-private))
        (source-collect-edges-decl-with-reviews (vector-get decl 1) nodes reviews edges)
        (if (= tag (ast-module-decl))
          (source-collect-edge-children-with-reviews
            decl
            5
            (vector-length decl)
            nodes
            reviews
            edges)
          (if (= tag (ast-impldef))
            (source-collect-edge-children-with-reviews
              decl
              4
              (vector-length decl)
              nodes
              reviews
              edges)
            (if (or (= tag (ast-typedef)) (= tag (ast-recorddef)))
              (let [forms (source-type-ordered-forms decl)]
                (source-append-edge-forms-with-reviews
                  forms
                  0
                  (vector-length forms)
                  nodes
                  reviews
                  edges))
              (source-result 1 edges))))))))

(defn source-collect-edges-program-loop-with-reviews [program idx len nodes reviews edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [parsed (source-collect-edges-decl-with-reviews
        (vector-get program idx)
        nodes
        reviews
        edges)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-edges-program-loop-with-reviews
          program
          (+ idx 1)
          len
          nodes
          reviews
          (source-result-value parsed))))))

(defn source-collect-edges-with-reviews [program nodes reviews]
  (source-collect-edges-program-loop-with-reviews
    program
    0
    (vector-length program)
    nodes
    reviews
    (vector-new 0)))

(defn source-collect-edges-program-loop [program idx len nodes edges]
  (source-collect-edges-program-loop-with-reviews
    program
    idx
    len
    nodes
    (vector-new 0)
    edges))

(defn source-collect-edges-decl [decl nodes edges]
  (source-collect-edges-decl-with-reviews decl nodes (vector-new 0) edges))

(defn source-collect-edge-children [decl idx len nodes edges]
  (source-collect-edge-children-with-reviews decl idx len nodes (vector-new 0) edges))

(defn source-collect-edges [program nodes]
  (source-collect-edges-with-reviews program nodes (vector-new 0)))

(defn source-graph-from-program [program]
  (let [nodes-result (source-collect-nodes program)]
    (if (= (source-result-status nodes-result) 0)
      nodes-result
      (let [nodes (source-result-value nodes-result)
        reviews-result (source-collect-reviews program)]
        (if (= (source-result-status reviews-result) 0)
          reviews-result
          (let [reviews (source-result-value reviews-result)
            edges-result (source-collect-edges-with-reviews program nodes reviews)]
            (if (= (source-result-status edges-result) 0)
              edges-result
              (source-result
                1
                (source-graph-with-reviews
                  nodes
                  (source-result-value edges-result)
                  reviews)))))))))
