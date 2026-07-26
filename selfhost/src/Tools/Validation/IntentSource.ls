(module Tools.Validation.IntentSource)
(import Syntax.AST)
(import Syntax.Parser)

;; M2 source metadata を selfhost の tagged vector graph へ投影する。
;;
;; record の wire shape は Rust の source adapter と同じ意味を持つ。
;; node: [kind, stable-id, text, span-start, span-end]
;; edge: [relation, left-id, right-id, span-start, span-end]
;; graph: [nodes, edges]
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

(defn source-error-malformed [] 1)
(defn source-error-invalid-id [] 2)
(defn source-error-kind-mismatch [] 3)
(defn source-error-duplicate-node [] 4)
(defn source-error-missing-node [] 5)
(defn source-error-evidence-registry-required [] 6)

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

(defn source-graph-nodes [graph] (vector-get graph 0))
(defn source-graph-edges [graph] (vector-get graph 1))

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

(defn source-node-kind? [kind]
  (and (>= kind (source-node-intent)) (<= kind (source-node-open-question))))

(defn source-edge-kind? [kind]
  (and (>= kind (source-edge-motivates)) (<= kind (source-edge-contradicts))))

(defn source-kind-prefix [kind]
  (if (= kind (source-node-intent)) "intent"
    (if (= kind (source-node-claim)) "claim"
      (if (= kind (source-node-assumption)) "assumption"
        (if (= kind (source-node-open-question)) "open-question"
          (if (= kind (source-edge-tested-by)) "contract"
            (if (or (= kind (source-edge-supports)) (= kind (source-edge-contradicts)))
              "evidence"
              "")))))))

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
            (if (or (= (string-length id) 0) (= (string-length text) 0))
              (source-result 0 (source-graph-error-at (source-error-malformed) kind id start end))
              (if (= (source-wire-valid? id kind) 0)
                (if (source-wire-valid-node? id)
                  (source-result 0 (source-graph-error-at (source-error-kind-mismatch) kind id start end))
                  (source-result 0 (source-graph-error-at (source-error-invalid-id) kind id start end)))
                (source-result 1 (source-node-record kind id text start end))))))))))

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

(defn source-append-edge-forms [forms idx len nodes edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [form (vector-get forms idx)
      kind (vector-get form 0)]
      (if (source-edge-kind? kind)
        (let [parsed (source-edge-form-result form nodes)]
          (if (= (source-result-status parsed) 0)
            parsed
            (source-append-edge-forms
              forms
              (+ idx 1)
              len
              nodes
              (vector-push-single-rooted-v3 edges (source-result-value parsed)))))
        (source-append-edge-forms forms (+ idx 1) len nodes edges)))))

(defn source-collect-edge-children [decl idx len nodes edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [child (vector-get decl idx)
      parsed (source-collect-edges-decl child nodes edges)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-edge-children decl (+ idx 1) len nodes (source-result-value parsed))))))

(defn source-collect-edges-decl [decl nodes edges]
  (let [tag (vector-get decl 0)]
    (if (= tag (ast-defn))
      (let [forms (source-ordered-forms decl)]
        (source-append-edge-forms forms 0 (vector-length forms) nodes edges))
      (if (= tag (ast-private))
        (source-collect-edges-decl (vector-get decl 1) nodes edges)
        (if (= tag (ast-module-decl))
          (source-collect-edge-children decl 5 (vector-length decl) nodes edges)
          (if (= tag (ast-impldef))
            (source-collect-edge-children decl 4 (vector-length decl) nodes edges)
            (if (or (= tag (ast-typedef)) (= tag (ast-recorddef)))
              (let [forms (source-type-ordered-forms decl)]
                (source-append-edge-forms forms 0 (vector-length forms) nodes edges))
              (source-result 1 edges))))))))

(defn source-collect-edges-program-loop [program idx len nodes edges]
  (if (>= idx len)
    (source-result 1 edges)
    (let [parsed (source-collect-edges-decl (vector-get program idx) nodes edges)]
      (if (= (source-result-status parsed) 0)
        parsed
        (source-collect-edges-program-loop
          program
          (+ idx 1)
          len
          nodes
          (source-result-value parsed))))))

(defn source-collect-edges [program nodes]
  (source-collect-edges-program-loop program 0 (vector-length program) nodes (vector-new 0)))

(defn source-graph-from-program [program]
  (let [nodes-result (source-collect-nodes program)]
    (if (= (source-result-status nodes-result) 0)
      nodes-result
      (let [nodes (source-result-value nodes-result)
        edges-result (source-collect-edges program nodes)]
        (if (= (source-result-status edges-result) 0)
          edges-result
          (source-result 1 (source-graph nodes (source-result-value edges-result))))))))
