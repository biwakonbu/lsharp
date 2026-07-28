# ADR: v0.2 native validation blank review provenance rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`, `crates/lsharp-types/src/validation_source/source_nodes.rs`, `crates/lsharp-types/tests/validation_source/nodes.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-invalid-review.md`

## Context

Review provenance must contain a non-blank digest. A whitespace-only value is not usable
provenance and must be rejected before review registry/report generation. The native smoke already
covered an unknown visibility value with code `8`, but not the separate non-blank digest check.
The selfhost source adapter checks required digest content before stable-ID shape. Rust source
adapter previously parsed the review ID first, so a malformed ID combined with a blank digest
returned the wire-format error instead of the required-field error.

## Decision

- Add `:review "review:checkout/blank-digest" "   " "redacted"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:8`, empty stdout, and no manifest file.
- Check a blank provenance digest before parsing the review ID. For `:review "review:checkout" "   "
  "redacted"`, return `SourceGraphError::InvalidReviewField { field: "provenance_digest" }`
  and the native/selfhost invalid-review code `8`.
- Keep blank digest rejection on the same invalid-review code as unknown visibility, distinct from
  invalid review IDs and duplicate reviews.

## Evidence

- RED: the Rust source test failed because a malformed review ID was parsed before the blank
  provenance digest; selfhost and native tests also lacked the combined-precedence fixture.
- GREEN: Rust source adapter now returns `InvalidReviewField { field: "provenance_digest" }`;
  selfhost actual Wasm returns status `0`, code `8`; native source-file smoke returns code `8`,
  exit `1`, with no report or manifest. Existing blank-digest and invalid-ID cases remain distinct.
- `bash -n`, native selfhost runner tests, docs audit, rustfmt, and `git diff --check` passed.

## Boundary and follow-up

This closes the blank review-digest required-field precedence across the native source-file smoke,
Rust source adapter, and selfhost direct consumer. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]`.
