(module Tools.Validation.Stale)
(import Tools.Validation.IntentSource)
(import Tools.Validation.Evidence)
(import Syntax.Parser)

;; Rust の IntentGraph::stale_subjects と同じ順序で、source graph の stale subject を
;; review/evidence の wire ID へ投影する。Cli と EmbeddedCli が同じ projection を使う。
(defn source-stale-id-exists-loop [ids id idx len]
  (if (>= idx len)
    0
    (if (string-eq (vector-get ids idx) id)
      1
      (source-stale-id-exists-loop ids id (+ idx 1) len))))
(defn source-stale-id-exists? [ids id]
  (source-stale-id-exists-loop ids id 0 (vector-length ids)))
(defn source-stale-add-id [ids id]
  (if (= (source-stale-id-exists? ids id) 1)
    ids
    (vector-push-single-rooted-v3 ids id)))
(defn source-stale-evidence-records-loop [registry idx len ids]
  (if (>= idx len)
    ids
    (let [stale-record (vector-get registry idx)
      next-ids
        (if (string-eq (source-evidence-record-outcome stale-record) "stale")
          (source-stale-add-id ids (source-evidence-record-id stale-record))
          ids)]
      (source-stale-evidence-records-loop registry (+ idx 1) len next-ids))))
(defn source-stale-invalidations-loop [edges idx len reviews evidence]
  (if (>= idx len)
    (vector-push-pair-rooted-v3 (vector-new 2) reviews evidence)
    (let [edge (vector-get edges idx)
      relation (source-edge-kind edge)
      subject (source-edge-right edge)
      next-reviews
        (if (and
              (= relation (source-edge-invalidates))
              (= (source-wire-valid? subject (source-review)) 1))
          (source-stale-add-id reviews subject)
          reviews)
      next-evidence
        (if (and
              (= relation (source-edge-invalidates))
              (= (source-wire-valid? subject (source-edge-supports)) 1))
          (source-stale-add-id evidence subject)
          evidence)]
      (source-stale-invalidations-loop edges (+ idx 1) len next-reviews next-evidence))))
(defn source-stale-evaluated-evidence-loop [edges review idx len evidence]
  (if (>= idx len)
    evidence
    (let [edge (vector-get edges idx)
      next-evidence
        (if (and
              (and
                (= (source-edge-kind edge) (source-edge-evaluates))
                (string-eq (source-edge-left edge) review))
              (= (source-wire-valid? (source-edge-right edge) (source-edge-supports)) 1))
          (source-stale-add-id evidence (source-edge-right edge))
          evidence)]
      (source-stale-evaluated-evidence-loop edges review (+ idx 1) len next-evidence))))
(defn source-stale-review-propagation-loop [edges reviews idx len evidence]
  (if (>= idx len)
    evidence
    (source-stale-review-propagation-loop
      edges
      reviews
      (+ idx 1)
      len
      (source-stale-evaluated-evidence-loop
        edges
        (vector-get reviews idx)
        0
        (vector-length edges)
        evidence))))
(defn source-evidence-stale-metrics [graph]
  (let [registry (source-evidence-graph-registry graph)
    edges (source-graph-edges graph)
    evidence0 (source-stale-evidence-records-loop registry 0 (vector-length registry) (vector-new 0))
    direct
      (source-stale-invalidations-loop edges 0 (vector-length edges) (vector-new 0) evidence0)
    reviews (vector-get direct 0)
    evidence1 (vector-get direct 1)
    evidence2
      (source-stale-review-propagation-loop
        edges
        reviews
        0
        (vector-length reviews)
        evidence1)
    metrics0 (vector-new 0)
    metrics1 (vector-push-single-rooted-v3 metrics0 (vector-length reviews))]
    (vector-push-single-rooted-v3 metrics1 (vector-length evidence2))))
