# ADR: v0.2 native validation review error span

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_edges.rs`, `crates/lsharp-types/tests/validation_source/edges.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-missing-review.md`、`docs/adr/decisions-v0.2-native-validation-invalidation-missing-review.md`

## Context

明示 review registry にない review を `evaluates` / `invalidates` が参照した場合、Rust source
adapter は `GraphError::MissingReview` へ直接落としていた。そのため code `10` は保てても
source directive の primary span が失われ、selfhost の source-local diagnostic と CLI の
source label が揃わなかった。

## Decision

- source adapter は `MissingReviewReference` を返し、relation、review ID、directive span を
  source-local error として保持する。
- `evaluates.review` と `invalidates.subject(kind=review)` の双方で同じ registry closure を
  適用する。
- review registry が空の場合は従来どおり external review identity を許可し、canonical
  `IntentGraph` の closure check は最終 safety net として残す。

## Evidence

- RED: source tests が `MissingReviewReference` を要求し、variant 未実装で compile error になった。
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`（52 tests）。
- Selfhost actual Wasm: missing `invalidates` review の code `10`、ID、directive span E2E と既存
  `evaluates` missing-review / invalidation subject-kind E2E が通過した。
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  が通過し、既存 invalidates missing-review fixtureの code `10`、no-report/no-manifest boundaryを
  維持した。
- Touched Rust files の `rustfmt --edition 2021 --check` と shell/provenance smoke は通過した。

## Boundary and follow-up

これは source adapter と selfhost diagnostic span の verified partial sliceである。driver の
`validate_source_review_edges` は current `origin/main` の EmbeddedCli build が
`selfhost/src/Tools/Validation/Stale.ls` の既存 `vector-push-single-rooted-v3` undefined error
で停止するため未検証とした。実 stage0 artifact/runtime、Mac/Linux matrix、native fallback exclusion、
provider authentication、review lifecycle、M2 aggregate completion は未完了であり、TODO の `[~]`
を維持する。
