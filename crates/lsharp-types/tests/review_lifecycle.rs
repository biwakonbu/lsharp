use lsharp_types::intent::review_lifecycle::{
    LifecycleError, ReviewLifecycleEvent, ReviewLifecycleRegistry, ReviewLifecycleState,
};

fn event(review_id: &str, sequence: u64, state: ReviewLifecycleState) -> ReviewLifecycleEvent {
    ReviewLifecycleEvent::new(
        review_id,
        sequence,
        state,
        "2026-08-01T00:00:00Z",
        None::<String>,
    )
    .expect("valid lifecycle event")
}

#[test]
fn lifecycle_reduces_allowed_transitions_and_orders_events_deterministically() {
    let mut registry = ReviewLifecycleRegistry::default();
    registry
        .add_event(event(
            "review:orders/reviewer-002",
            1,
            ReviewLifecycleState::Active,
        ))
        .expect("initial active state is allowed");
    registry
        .add_event(event(
            "review:orders/reviewer-001",
            1,
            ReviewLifecycleState::Proposed,
        ))
        .expect("initial proposed state is allowed");
    registry
        .add_event(event(
            "review:orders/reviewer-001",
            2,
            ReviewLifecycleState::Active,
        ))
        .expect("proposed to active is allowed");
    registry
        .add_event(event(
            "review:orders/reviewer-001",
            3,
            ReviewLifecycleState::Revoked,
        ))
        .expect("active to revoked is allowed");

    assert_eq!(
        registry.state_for("review:orders/reviewer-001"),
        Some(ReviewLifecycleState::Revoked)
    );
    assert_eq!(
        registry
            .events()
            .iter()
            .map(|event| event.review_id().as_str())
            .collect::<Vec<_>>(),
        vec![
            "review:orders/reviewer-001",
            "review:orders/reviewer-001",
            "review:orders/reviewer-001",
            "review:orders/reviewer-002",
        ]
    );
}

#[test]
fn lifecycle_rejects_invalid_first_event_and_empty_effective_time() {
    assert!(matches!(
        ReviewLifecycleEvent::new(
            "review:orders/reviewer-001",
            0,
            ReviewLifecycleState::Proposed,
            "2026-08-01T00:00:00Z",
            None::<String>,
        ),
        Err(LifecycleError::InvalidSequence { sequence: 0 })
    ));
    assert!(matches!(
        ReviewLifecycleEvent::new(
            "review:orders/reviewer-001",
            1,
            ReviewLifecycleState::Proposed,
            "   ",
            None::<String>,
        ),
        Err(LifecycleError::EmptyField {
            field: "effective_at"
        })
    ));
}

#[test]
fn lifecycle_rejects_malformed_effective_timestamp() {
    let result = ReviewLifecycleEvent::new(
        "review:orders/reviewer-001",
        1,
        ReviewLifecycleState::Proposed,
        "not-a-canonical-timestamp",
        None::<String>,
    );

    assert!(matches!(
        result,
        Err(LifecycleError::InvalidTimestamp {
            field: "effective_at",
            value
        }) if value == "not-a-canonical-timestamp"
    ));
}

#[test]
fn lifecycle_rejects_duplicate_or_rollback_sequences_and_invalid_transitions() {
    let mut registry = ReviewLifecycleRegistry::default();
    registry
        .add_event(event(
            "review:orders/reviewer-001",
            2,
            ReviewLifecycleState::Active,
        ))
        .expect("initial active state is allowed");

    assert!(matches!(
        registry.add_event(event(
            "review:orders/reviewer-001",
            1,
            ReviewLifecycleState::Revoked,
        )),
        Err(LifecycleError::SequenceRollback { .. })
    ));
    assert!(matches!(
        registry.add_event(event(
            "review:orders/reviewer-001",
            2,
            ReviewLifecycleState::Active,
        )),
        Err(LifecycleError::DuplicateSequence { .. })
    ));
    assert!(matches!(
        registry.add_event(event(
            "review:orders/reviewer-001",
            3,
            ReviewLifecycleState::Proposed,
        )),
        Err(LifecycleError::InvalidTransition { .. })
    ));
}

#[test]
fn lifecycle_rejects_effective_time_rollback() {
    let mut registry = ReviewLifecycleRegistry::default();
    registry
        .add_event(
            ReviewLifecycleEvent::new(
                "review:orders/reviewer-001",
                1,
                ReviewLifecycleState::Proposed,
                "2026-08-02T00:00:00Z",
                None,
            )
            .expect("valid initial event"),
        )
        .expect("initial event should be accepted");

    let result = registry.add_event(
        ReviewLifecycleEvent::new(
            "review:orders/reviewer-001",
            2,
            ReviewLifecycleState::Active,
            "2026-08-01T23:59:59Z",
            None,
        )
        .expect("valid event shape"),
    );

    assert!(matches!(
        result,
        Err(LifecycleError::EffectiveTimeRollback {
            review_id,
            previous,
            next,
        }) if review_id == "review:orders/reviewer-001"
            && previous == "2026-08-02T00:00:00Z"
            && next == "2026-08-01T23:59:59Z"
    ));
}

#[test]
fn lifecycle_rejects_resurrection_after_terminal_state() {
    let mut registry = ReviewLifecycleRegistry::default();
    registry
        .add_event(event(
            "review:orders/reviewer-001",
            1,
            ReviewLifecycleState::Active,
        ))
        .expect("initial active state is allowed");
    registry
        .add_event(event(
            "review:orders/reviewer-001",
            2,
            ReviewLifecycleState::Superseded,
        ))
        .expect("active to superseded is allowed");

    assert!(matches!(
        registry.add_event(event(
            "review:orders/reviewer-001",
            3,
            ReviewLifecycleState::Active,
        )),
        Err(LifecycleError::InvalidTransition { .. })
    ));
}
