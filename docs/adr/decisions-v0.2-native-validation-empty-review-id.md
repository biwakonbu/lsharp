# ADR: v0.2 native validation empty review ID rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`, `crates/lsharp-types/src/validation_source/source_nodes.rs`, `crates/lsharp-types/tests/validation_source/nodes.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-invalid-review-id.md`

## Context

Review metadata requires a stable ID. An empty review ID is a missing required field, not merely a
malformed wire shape, and must fail before review registry/report generation. The native smoke now
covers malformed review IDs and invalid provenance as well as the empty-ID branch.

The native source-file smoke contract treats an empty `:review` ID as required review metadata
failure code `8`. Selfhost checks an empty review ID before stable-ID parsing and returns the same
code. Rust source adapter previously called `ReviewId::parse("")` first, exposing a generic
wire-format boundary (`ReviewIdAt`) that did not match the native/selfhost contract.

## Decision

- Add `:review "" "sha256:review-provenance" "redacted"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:8`, empty stdout, and no manifest file.
- Reject an empty review ID in the Rust source adapter as
  `SourceGraphError::InvalidReviewField { field: "id", value: "" }`, before stable-ID parsing.
- Keep empty review ID rejection distinct from malformed review IDs (`code 2`) and blank digest or
  unknown visibility invalid-review fields.

## Evidence

- RED: native smoke initially failed because the empty-review-ID fixture and contract were absent
  from the inner smoke; the Rust source test then failed because the empty ID did not match
  `InvalidReviewField`.
- GREEN: focused native command returned `Linux native stage0 source-file provenance tests: OK`;
  Rust source test now returns `InvalidReviewField { field: "id" }`; selfhost actual Wasm returns
  status `0`, code `8`; the native source-file smoke fixture returns code `8`, exit `1`, with no
  report or manifest.
- `bash -n`, native selfhost runner tests, docs audit, rustfmt, and `git diff --check` passed.

## Boundary and follow-up

This closes the empty review-ID native source-file, Rust source adapter, and selfhost direct-consumer
boundary. It does not prove current-source packaged stage0 execution, Mac/Linux artifact/runtime
parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`, `EC-M2-03`, and M3 aggregate `[~]`.
