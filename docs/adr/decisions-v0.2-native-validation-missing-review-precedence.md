# ADR: v0.2 native validation missing-review precedence

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_edges.rs`, `crates/lsharp-types/tests/validation_source/edges.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`, `scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-missing-review.md`、`docs/adr/decisions-v0.2-native-validation-review-subject-kind.md`

## Context

When an explicit review registry exists, selfhost `IntentSource` checks that the left review ID of
an `evaluates` edge is registered before validating the subject. Rust parsed the subject first, so a
missing review combined with a forbidden review subject returned `EdgeSubjectKindMismatch` (code `9`)
instead of `MissingReview` (code `10`). The same source then had different diagnostics depending on
which implementation consumed it.

## Decision

- Parse the `evaluates` review ID as before, then require it in the explicit review registry before
  parsing or validating the subject.
- Keep an empty review registry open for external review identities; only an explicit non-empty
  registry triggers `MissingReview`.
- Preserve code `10`, exit `1`, empty stdout, and no manifest for the combined missing-review/
  invalid-subject fixture.

## Evidence

- RED: the Rust source test returned `EdgeSubjectKindMismatch` for a missing review plus a review
  subject, while the expected native boundary was `GraphError::MissingReview`.
- Rust GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture` passed (51 tests).
- Rust-host selfhost GREEN:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_intent_source_adapter::test_e2e_selfhost_source_adapter_reports_missing_review_before_invalid_evaluates_subject -- --exact --nocapture` passed.
- Native contract: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` passed
  under the fake Lima/provenance harness and checks the code `10` no-report/no-manifest boundary.
- `bash scripts/audit_docs.sh`、`bash -n`、`git diff --check` are required final gates for this slice.

## Boundary and follow-up

This closes only the `evaluates` missing-review precedence boundary. It does not prove packaged
current-source stage0 execution, Mac/Linux artifact/runtime parity, native fallback exclusion, or
the full review lifecycle/authentication contract. Keep `EC-M2-02` and the M3 aggregate `[~]`.
