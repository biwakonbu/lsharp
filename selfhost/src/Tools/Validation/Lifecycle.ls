(module Tools.Validation.Lifecycle)
(import Syntax.Parser)
(import Tools.Validation.IntentSource)
(import Tools.Validation.Whitespace)

;; v0.3 review lifecycle の selfhost reducer。
;; event: [review-id, sequence, state, effective-at, reason-digest-or-empty]
;; registry: deterministic orderへ正規化された event vector

(defn source-review-lifecycle-proposed [] "proposed")
(defn source-review-lifecycle-active [] "active")
(defn source-review-lifecycle-superseded [] "superseded")
(defn source-review-lifecycle-revoked [] "revoked")

(defn source-review-lifecycle-error-malformed [] 1)
(defn source-review-lifecycle-error-invalid-review-id [] 2)
(defn source-review-lifecycle-error-invalid-field [] 3)
(defn source-review-lifecycle-error-duplicate-sequence [] 4)
(defn source-review-lifecycle-error-sequence-rollback [] 5)
(defn source-review-lifecycle-error-invalid-initial-state [] 6)
(defn source-review-lifecycle-error-invalid-transition [] 7)

(defn source-review-lifecycle-event
  [review-id sequence state effective-at reason-digest]
  (let [base (vector-push-quad-rooted-v3
      (vector-new 1)
      review-id
      sequence
      state
      effective-at)]
    (vector-push-single-rooted-v3 base reason-digest)))

(defn source-review-lifecycle-event-review-id [event] (vector-get event 0))
(defn source-review-lifecycle-event-sequence [event] (vector-get event 1))
(defn source-review-lifecycle-event-state [event] (vector-get event 2))
(defn source-review-lifecycle-event-effective-at [event] (vector-get event 3))
(defn source-review-lifecycle-event-reason-digest [event] (vector-get event 4))

(defn source-review-lifecycle-error
  [code event previous-sequence previous-state]
  (let [base (vector-push-quad-rooted-v3
      (vector-new 1)
      code
      (source-review-lifecycle-event-review-id event)
      (source-review-lifecycle-event-sequence event)
      previous-sequence)
    with-state (vector-push-single-rooted-v3
      base
      (source-review-lifecycle-event-state event))]
    (vector-push-single-rooted-v3 with-state previous-state)))

(defn source-review-lifecycle-error-code [error] (vector-get error 0))
(defn source-review-lifecycle-error-review-id [error] (vector-get error 1))
(defn source-review-lifecycle-error-sequence [error] (vector-get error 2))
(defn source-review-lifecycle-error-previous-sequence [error] (vector-get error 3))
(defn source-review-lifecycle-error-state [error] (vector-get error 4))
(defn source-review-lifecycle-error-previous-state [error] (vector-get error 5))

(defn source-review-lifecycle-state-valid? [state]
  (or
    (or
      (string-eq state (source-review-lifecycle-proposed))
      (string-eq state (source-review-lifecycle-active)))
    (or
      (string-eq state (source-review-lifecycle-superseded))
      (string-eq state (source-review-lifecycle-revoked)))))

(defn source-review-lifecycle-event-valid? [event]
  (if (!= (vector-length event) 5)
    0
    (let [review-id (source-review-lifecycle-event-review-id event)
      sequence (source-review-lifecycle-event-sequence event)
      state (source-review-lifecycle-event-state event)
      effective-at (source-review-lifecycle-event-effective-at event)
      reason-digest (source-review-lifecycle-event-reason-digest event)]
      (if (= (validation-nonblank? review-id) 0)
        0
        (if (= (source-wire-valid? review-id (source-review)) 0)
          0
          (if (< sequence 1)
            0
            (if (source-review-lifecycle-state-valid? state)
              (if (= (source-review-attestation-timestamp-valid? effective-at) 0)
                0
                (if
                  (and
                    (> (string-length reason-digest) 0)
                    (= (validation-nonblank? reason-digest) 0))
                  0
                  1))
              0)))))))

(defn source-review-lifecycle-validate-event [event]
  (if (!= (vector-length event) 5)
    (source-result
      0
      (source-review-lifecycle-error
        (source-review-lifecycle-error-malformed)
        (source-review-lifecycle-event "" 0 "" "" "")
        -1
        ""))
    (if (= (validation-nonblank? (source-review-lifecycle-event-review-id event)) 0)
      (source-result
        0
        (source-review-lifecycle-error
          (source-review-lifecycle-error-invalid-field)
          event
          -1
          ""))
      (if (= (source-wire-valid?
                (source-review-lifecycle-event-review-id event)
                (source-review))
              0)
        (source-result
          0
          (source-review-lifecycle-error
            (source-review-lifecycle-error-invalid-review-id)
            event
            -1
            ""))
        (if (= (source-review-lifecycle-event-valid? event) 0)
          (source-result
            0
            (source-review-lifecycle-error
              (source-review-lifecycle-error-invalid-field)
              event
              -1
              ""))
          (source-result 1 event))))))

;; string-char-at/string-length は UTF-8 bytes を返すため、Rust の str::cmp と同じ
;; byte-wise lexicographic order を明示する。
(defn source-review-lifecycle-string-before-loop [left right idx limit]
  (if (>= idx limit)
    (if (< (string-length left) (string-length right)) 1 0)
    (let [left-byte (string-char-at left idx)
      right-byte (string-char-at right idx)]
      (if (< left-byte right-byte)
        1
        (if (> left-byte right-byte)
          0
          (source-review-lifecycle-string-before-loop
            left
            right
            (+ idx 1)
            limit))))))

(defn source-review-lifecycle-string-before? [left right]
  (let [left-len (string-length left)
    right-len (string-length right)
    limit (if (< left-len right-len) left-len right-len)]
    (if (string-eq left right)
      0
      (source-review-lifecycle-string-before-loop left right 0 limit))))

(defn source-review-lifecycle-event-before? [left right]
  (let [left-id (source-review-lifecycle-event-review-id left)
    right-id (source-review-lifecycle-event-review-id right)]
    (if (string-eq left-id right-id)
      (if (<
            (source-review-lifecycle-event-sequence left)
            (source-review-lifecycle-event-sequence right))
        1
        0)
      (source-review-lifecycle-string-before? left-id right-id))))

(defn source-review-lifecycle-copy [src from to out]
  (if (>= from to)
    out
    (source-review-lifecycle-copy
      src
      (+ from 1)
      to
      (vector-push-single-rooted-v3 out (vector-get src from)))))

(defn source-review-lifecycle-insert [sorted elem idx]
  (if (= idx 0)
    (let [out0 (vector-new (+ (vector-length sorted) 1))
      out (vector-push-single-rooted-v3 out0 elem)]
      (source-review-lifecycle-copy sorted 0 (vector-length sorted) out))
    (let [previous (vector-get sorted (- idx 1))]
      (if (= (source-review-lifecycle-event-before? elem previous) 1)
        (source-review-lifecycle-insert sorted elem (- idx 1))
        (let [out (vector-new (+ (vector-length sorted) 1))
          copied (source-review-lifecycle-copy sorted 0 idx out)
          with-elem (vector-push-single-rooted-v3 copied elem)]
          (source-review-lifecycle-copy sorted idx (vector-length sorted) with-elem))))))

(defn source-review-lifecycle-sort-loop [events sorted idx len]
  (if (>= idx len)
    sorted
    (source-review-lifecycle-sort-loop
      events
      (source-review-lifecycle-insert
        sorted
        (vector-get events idx)
        (vector-length sorted))
      (+ idx 1)
      len)))

(defn source-review-lifecycle-sort [events]
  (let [len (vector-length events)]
    (if (< len 2)
      events
      (let [initial (vector-push-single-rooted-v3
          (vector-new 1)
          (vector-get events 0))]
        (source-review-lifecycle-sort-loop events initial 1 len)))))

(defn source-review-lifecycle-current-loop [registry review-id idx len current]
  (if (>= idx len)
    current
    (let [event (vector-get registry idx)]
      (if (string-eq
            (source-review-lifecycle-event-review-id event)
            review-id)
        (if
          (or
            (= current 0)
            (> (source-review-lifecycle-event-sequence event)
              (source-review-lifecycle-event-sequence current)))
          (source-review-lifecycle-current-loop
            registry
            review-id
            (+ idx 1)
            len
            event)
          (source-review-lifecycle-current-loop
            registry
            review-id
            (+ idx 1)
            len
            current))
        (source-review-lifecycle-current-loop
          registry
          review-id
          (+ idx 1)
          len
          current)))))

(defn source-review-lifecycle-current [registry review-id]
  (source-review-lifecycle-current-loop
    registry
    review-id
    0
    (vector-length registry)
    0))

(defn source-review-lifecycle-transition-valid? [from to]
  (or
    (and
      (string-eq from (source-review-lifecycle-proposed))
      (string-eq to (source-review-lifecycle-active)))
    (and
      (string-eq from (source-review-lifecycle-active))
      (or
        (string-eq to (source-review-lifecycle-superseded))
        (string-eq to (source-review-lifecycle-revoked))))))

(defn source-review-lifecycle-add-event [registry event]
  (let [validated (source-review-lifecycle-validate-event event)]
    (if (= (source-result-status validated) 0)
      validated
      (let [review-id (source-review-lifecycle-event-review-id event)
        current (source-review-lifecycle-current registry review-id)]
        (if (= current 0)
          (if
            (or
              (string-eq
                (source-review-lifecycle-event-state event)
                (source-review-lifecycle-proposed))
              (string-eq
                (source-review-lifecycle-event-state event)
                (source-review-lifecycle-active)))
            (source-result
              1
              (vector-push-single-rooted-v3 registry event))
            (source-result
              0
              (source-review-lifecycle-error
                (source-review-lifecycle-error-invalid-initial-state)
                event
                -1
                "")))
          (let [previous-sequence (source-review-lifecycle-event-sequence current)
            sequence (source-review-lifecycle-event-sequence event)
            previous-state (source-review-lifecycle-event-state current)
            state (source-review-lifecycle-event-state event)]
            (if (= sequence previous-sequence)
              (source-result
                0
                (source-review-lifecycle-error
                  (source-review-lifecycle-error-duplicate-sequence)
                  event
                  previous-sequence
                  previous-state))
              (if (< sequence previous-sequence)
                (source-result
                  0
                  (source-review-lifecycle-error
                    (source-review-lifecycle-error-sequence-rollback)
                    event
                    previous-sequence
                    previous-state))
                (if (source-review-lifecycle-transition-valid? previous-state state)
                  (source-result
                    1
                    (vector-push-single-rooted-v3 registry event))
                  (source-result
                    0
                    (source-review-lifecycle-error
                      (source-review-lifecycle-error-invalid-transition)
                      event
                      previous-sequence
                      previous-state)))))))))))

(defn source-review-lifecycle-add-events-loop [events idx len registry]
  (if (>= idx len)
    (source-result 1 (source-review-lifecycle-sort registry))
    (let [result (source-review-lifecycle-add-event
        registry
        (vector-get events idx))]
      (if (= (source-result-status result) 0)
        result
        (source-review-lifecycle-add-events-loop
          events
          (+ idx 1)
          len
          (source-result-value result))))))

(defn source-review-lifecycle-validate-events-loop [events idx len]
  (if (>= idx len)
    (source-result 1 events)
    (let [result (source-review-lifecycle-validate-event (vector-get events idx))]
      (if (= (source-result-status result) 0)
        result
        (source-review-lifecycle-validate-events-loop events (+ idx 1) len)))))

(defn source-review-lifecycle-from-events [events]
  (let [validated (source-review-lifecycle-validate-events-loop
      events
      0
      (vector-length events))]
    (if (= (source-result-status validated) 0)
      validated
      (let [sorted (source-review-lifecycle-sort (source-result-value validated))]
        (source-review-lifecycle-add-events-loop
          sorted
          0
          (vector-length sorted)
          (source-review-lifecycle-new))))))

(defn source-review-lifecycle-new [] (vector-new 0))

(defn source-review-lifecycle-events [registry]
  (source-review-lifecycle-sort registry))

(defn source-review-lifecycle-state-for-loop [registry review-id idx len current]
  (if (>= idx len)
    current
    (let [event (vector-get registry idx)]
      (if (string-eq
            (source-review-lifecycle-event-review-id event)
            review-id)
        (source-review-lifecycle-state-for-loop
          registry
          review-id
          (+ idx 1)
          len
          (source-review-lifecycle-event-state event))
        (source-review-lifecycle-state-for-loop
          registry
          review-id
          (+ idx 1)
          len
          current)))))

(defn source-review-lifecycle-state-for [registry review-id]
  (source-review-lifecycle-state-for-loop
    (source-review-lifecycle-sort registry)
    review-id
    0
    (vector-length registry)
    ""))
