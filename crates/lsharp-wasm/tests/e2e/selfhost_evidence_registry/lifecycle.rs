//! selfhost lifecycle reducer と Rust lifecycle model の parity tests。

use super::super::support::*;

fn run_lifecycle_runtime(harness: &str) -> String {
    let intent_source = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/IntentSource.ls"),
    )
    .expect("canonical IntentSource.ls が読み込めない");
    let whitespace = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Whitespace.ls"),
    )
    .expect("canonical Whitespace.ls が読み込めない");
    let lifecycle = std::fs::read_to_string(
        selfhost_project_root().join("selfhost/src/Tools/Validation/Lifecycle.ls"),
    )
    .expect("canonical Lifecycle.ls が読み込めない");
    compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        whitespace,
        intent_source,
        lifecycle,
        harness
    ))
}

#[test]
fn selfhost_lifecycle_reducer_orders_events_and_rejects_invalid_transitions() {
    let harness = r#"
(defn event-at [review-id sequence state effective-at]
  (source-review-lifecycle-event
    review-id
    sequence
    state
    effective-at
    ""))
(defn event [review-id sequence state]
  (event-at review-id sequence state "2026-08-01T00:00:00Z"))
(defn push-event [events elem]
  (vector-push-single-rooted-v3 events elem))
(defn main []
  (let [out-of-order
          (push-event
            (push-event
              (push-event
                (push-event
                  (vector-new 4)
                  (event "review:orders/reviewer-002" 1 "active"))
                (event "review:orders/reviewer-001" 3 "revoked"))
              (event "review:orders/reviewer-001" 1 "proposed"))
            (event "review:orders/reviewer-001" 2 "active"))
        reduced (source-review-lifecycle-from-events out-of-order)
        registry (source-result-value reduced)
        ordered (source-review-lifecycle-events registry)
        duplicate
          (source-review-lifecycle-from-events
            (push-event
              (push-event (vector-new 2) (event "review:orders/reviewer-001" 1 "active"))
              (event "review:orders/reviewer-001" 1 "active")))
        invalid-initial
          (source-review-lifecycle-from-events
            (push-event (vector-new 1) (event "review:orders/reviewer-001" 1 "revoked")))
        first-add
          (source-review-lifecycle-add-event
            (source-review-lifecycle-new)
            (event "review:orders/reviewer-001" 2 "active"))
        rollback
          (source-review-lifecycle-add-event
            (source-result-value first-add)
            (event "review:orders/reviewer-001" 1 "active"))
        effective-time-rollback
          (source-review-lifecycle-add-event
            (source-result-value first-add)
            (event-at
              "review:orders/reviewer-001"
              3
              "superseded"
              "2026-07-31T23:59:59Z"))
        first-terminal
          (source-review-lifecycle-add-event
            (source-result-value first-add)
            (event "review:orders/reviewer-001" 3 "superseded"))
        resurrection
          (source-review-lifecycle-add-event
            (source-result-value first-terminal)
            (event "review:orders/reviewer-001" 4 "active"))]
    (do
      (print (source-result-status reduced))
      (print (vector-length ordered))
      (print-string (source-review-lifecycle-event-state (vector-get ordered 0)))
      (print-string "\n")
      (print-string (source-review-lifecycle-event-state (vector-get ordered 1)))
      (print-string "\n")
      (print-string (source-review-lifecycle-event-state (vector-get ordered 2)))
      (print-string "\n")
      (print-string (source-review-lifecycle-event-review-id (vector-get ordered 3)))
      (print-string "\n")
      (print (source-review-lifecycle-event-sequence (vector-get ordered 0)))
      (print (source-review-lifecycle-event-sequence (vector-get ordered 1)))
      (print (source-review-lifecycle-event-sequence (vector-get ordered 2)))
      (print (source-review-lifecycle-event-sequence (vector-get ordered 3)))
      (print (source-result-status duplicate))
      (print (source-review-lifecycle-error-code (source-result-error duplicate)))
      (print (source-result-status invalid-initial))
      (print (source-review-lifecycle-error-code (source-result-error invalid-initial)))
      (print (source-result-status rollback))
      (print (source-review-lifecycle-error-code (source-result-error rollback)))
      (print (source-result-status effective-time-rollback))
      (print
        (source-review-lifecycle-error-code
          (source-result-error effective-time-rollback)))
      (print-string
        (source-review-lifecycle-error-effective-at
          (source-result-error effective-time-rollback)))
      (print-string "\n")
      (print-string
        (source-review-lifecycle-error-previous-effective-at
          (source-result-error effective-time-rollback)))
      (print-string "\n")
      (print (source-result-status resurrection))
      (print (source-review-lifecycle-error-code (source-result-error resurrection)))
      0)))
"#;

    let output = run_lifecycle_runtime(harness);
    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        [
            "1",
            "4",
            "proposed",
            "active",
            "revoked",
            "review:orders/reviewer-002",
            "1",
            "2",
            "3",
            "1",
            "0",
            "4",
            "0",
            "6",
            "0",
            "5",
            "0",
            "8",
            "2026-07-31T23:59:59Z",
            "2026-08-01T00:00:00Z",
            "0",
            "7",
        ]
    );
}

#[test]
fn selfhost_lifecycle_event_at_selects_latest_event_not_future() {
    let harness = r#"
(defn event-at [review-id sequence state effective-at]
  (source-review-lifecycle-event
    review-id
    sequence
    state
    effective-at
    ""))
(defn push-event [events elem]
  (vector-push-single-rooted-v3 events elem))
(defn main []
  (let [events
          (push-event
            (push-event
              (push-event
                (vector-new 3)
                (event-at
                  "review:clock/reviewer-001"
                  3
                  "revoked"
                  "2026-08-03T00:00:00Z"))
              (event-at
                "review:clock/reviewer-001"
                1
                "proposed"
                "2026-08-01T00:00:00Z"))
            (event-at
              "review:clock/reviewer-001"
              2
              "active"
              "2026-08-02T00:00:00Z"))
        reduced (source-review-lifecycle-from-events events)
        registry (source-result-value reduced)
        before
          (source-review-lifecycle-event-at
            registry
            "review:clock/reviewer-001"
            "2026-07-31T23:59:59Z")
        first
          (source-review-lifecycle-event-at
            registry
            "review:clock/reviewer-001"
            "2026-08-01T12:00:00Z")
        active
          (source-review-lifecycle-event-at
            registry
            "review:clock/reviewer-001"
            "2026-08-02T00:00:00Z")
        revoked
          (source-review-lifecycle-event-at
            registry
            "review:clock/reviewer-001"
            "2026-08-04T00:00:00Z")]
    (do
      (print (if (= before 0) 1 0))
      (print (source-review-lifecycle-event-sequence first))
      (print (source-review-lifecycle-event-sequence active))
      (print (source-review-lifecycle-event-sequence revoked))
      0)))
"#;

    let output = run_lifecycle_runtime(harness);
    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        ["1", "1", "2", "3"]
    );
}
