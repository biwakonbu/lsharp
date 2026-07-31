//! v0.3 review provenance の append-only lifecycle reducer。
//!
//! provider snapshot の event を review ID ごとに sequence 順へ固定し、失効・差し替え後の
//! resurrection や sequence の巻き戻しを暗黙に許さない。外部 provider の取得、署名検証、
//! clock 判定はこの module の責務ではなく、検証済み event を入力する後続 boundary とする。

use super::{ReviewId, StableIdError};
use std::collections::BTreeMap;

/// review lifecycle の状態。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReviewLifecycleState {
    Proposed,
    Active,
    Superseded,
    Revoked,
}

impl ReviewLifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::Revoked => "revoked",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Superseded | Self::Revoked)
    }

    const fn allows_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Proposed, Self::Active)
                | (Self::Active, Self::Superseded)
                | (Self::Active, Self::Revoked)
        )
    }
}

/// lifecycle event が不正である理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleError {
    #[error("review lifecycle の必須 field が空です: {field}")]
    EmptyField { field: &'static str },
    #[error("review lifecycle の timestamp が不正です: field={field}, value={value:?}")]
    InvalidTimestamp {
        field: &'static str,
        value: String,
    },
    #[error("review lifecycle の sequence は 1 以上でなければなりません: {sequence}")]
    InvalidSequence { sequence: u64 },
    #[error("review lifecycle の review ID が不正です: {0}")]
    InvalidReviewId(#[from] StableIdError),
    #[error("review lifecycle の初期 state が不正です: {state:?}")]
    InvalidInitialState { state: ReviewLifecycleState },
    #[error(
        "review lifecycle の sequence が巻き戻っています: review_id={review_id:?}, previous={previous}, next={next}"
    )]
    SequenceRollback {
        review_id: String,
        previous: u64,
        next: u64,
    },
    #[error(
        "review lifecycle の effective_at が巻き戻っています: review_id={review_id:?}, previous={previous:?}, next={next:?}"
    )]
    EffectiveTimeRollback {
        review_id: String,
        previous: String,
        next: String,
    },
    #[error("review lifecycle に同じ sequence が重複しています: review_id={review_id:?}, sequence={sequence}")]
    DuplicateSequence { review_id: String, sequence: u64 },
    #[error(
        "review lifecycle の遷移が不正です: review_id={review_id:?}, from={from:?}, to={to:?}"
    )]
    InvalidTransition {
        review_id: String,
        from: ReviewLifecycleState,
        to: ReviewLifecycleState,
    },
}

/// review ID に対する append-only lifecycle event。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewLifecycleEvent {
    review_id: ReviewId,
    sequence: u64,
    state: ReviewLifecycleState,
    effective_at: String,
    reason_digest: Option<String>,
}

impl ReviewLifecycleEvent {
    pub fn new(
        review_id: impl Into<String>,
        sequence: u64,
        state: ReviewLifecycleState,
        effective_at: impl Into<String>,
        reason_digest: Option<String>,
    ) -> Result<Self, LifecycleError> {
        let review_id = review_id.into();
        let effective_at = effective_at.into();
        if sequence == 0 {
            return Err(LifecycleError::InvalidSequence { sequence });
        }
        validate_required("review_id", &review_id)?;
        validate_required("effective_at", &effective_at)?;
        if !super::review_attestation::canonical_timestamp_is_valid(&effective_at) {
            return Err(LifecycleError::InvalidTimestamp {
                field: "effective_at",
                value: effective_at,
            });
        }
        if let Some(reason_digest) = &reason_digest {
            validate_required("reason_digest", reason_digest)?;
        }
        let review_id = ReviewId::parse(review_id).map_err(LifecycleError::InvalidReviewId)?;
        Ok(Self {
            review_id,
            sequence,
            state,
            effective_at,
            reason_digest,
        })
    }

    pub fn review_id(&self) -> &ReviewId {
        &self.review_id
    }

    pub const fn sequence(&self) -> u64 {
        self.sequence
    }

    pub const fn state(&self) -> ReviewLifecycleState {
        self.state
    }

    pub fn effective_at(&self) -> &str {
        &self.effective_at
    }

    pub fn reason_digest(&self) -> Option<&str> {
        self.reason_digest.as_deref()
    }
}

/// review lifecycle event を deterministic に reduce する registry。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewLifecycleRegistry {
    events: BTreeMap<String, Vec<ReviewLifecycleEvent>>,
}

impl ReviewLifecycleRegistry {
    /// 宣言順に依存しない lifecycle snapshot を deterministic に構築する。
    ///
    /// wire や provider snapshot は event の配列順を保証しないため、review ID と sequence
    /// の tuple で先に並べ替える。個別の `add_event` は append-only boundary として残し、
    /// 並べ替え後も duplicate、rollback、invalid transition は同じ fail-closed error を返す。
    pub fn from_events(
        events: impl IntoIterator<Item = ReviewLifecycleEvent>,
    ) -> Result<Self, LifecycleError> {
        let mut events = events.into_iter().collect::<Vec<_>>();
        events.sort_by(|left, right| {
            left.review_id()
                .as_str()
                .cmp(right.review_id().as_str())
                .then_with(|| left.sequence().cmp(&right.sequence()))
        });

        let mut registry = Self::default();
        for event in events {
            registry.add_event(event)?;
        }
        Ok(registry)
    }

    /// 新しい event を review ID ごとの末尾へ追加する。
    pub fn add_event(&mut self, event: ReviewLifecycleEvent) -> Result<(), LifecycleError> {
        let review_id = event.review_id().as_str().to_string();
        let history = self.events.entry(review_id.clone()).or_default();
        if let Some(previous) = history.last() {
            if event.sequence() == previous.sequence() {
                return Err(LifecycleError::DuplicateSequence {
                    review_id,
                    sequence: event.sequence(),
                });
            }
            if event.sequence() < previous.sequence() {
                return Err(LifecycleError::SequenceRollback {
                    review_id,
                    previous: previous.sequence(),
                    next: event.sequence(),
                });
            }
            // event construction で固定長の canonical UTC へ検証済みなので、文字列順は時系列順と一致する。
            if event.effective_at() < previous.effective_at() {
                return Err(LifecycleError::EffectiveTimeRollback {
                    review_id,
                    previous: previous.effective_at().to_string(),
                    next: event.effective_at().to_string(),
                });
            }
            if !previous.state().allows_transition_to(event.state()) {
                return Err(LifecycleError::InvalidTransition {
                    review_id,
                    from: previous.state(),
                    to: event.state(),
                });
            }
        } else if !matches!(
            event.state(),
            ReviewLifecycleState::Proposed | ReviewLifecycleState::Active
        ) {
            return Err(LifecycleError::InvalidInitialState {
                state: event.state(),
            });
        }
        history.push(event);
        Ok(())
    }

    /// review ID の現在 state を返す。
    pub fn state_for(&self, review_id: &str) -> Option<ReviewLifecycleState> {
        self.events
            .get(review_id)
            .and_then(|events| events.last())
            .map(ReviewLifecycleEvent::state)
    }

    /// review ID の現在 event を返す。state と sequence の組を同時に検証する caller 向け。
    pub fn current_event_for(&self, review_id: &str) -> Option<&ReviewLifecycleEvent> {
        self.events.get(review_id).and_then(|events| events.last())
    }

    /// review ID、sequence の順に flatten した deterministic view を返す。
    pub fn events(&self) -> Vec<&ReviewLifecycleEvent> {
        self.events
            .values()
            .flat_map(|events| events.iter())
            .collect()
    }
}

fn validate_required(field: &'static str, value: &str) -> Result<(), LifecycleError> {
    if value.trim().is_empty() {
        return Err(LifecycleError::EmptyField { field });
    }
    Ok(())
}
