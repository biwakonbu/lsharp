# ADR: v0.2 native validation whitespace-only evidence subject

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Evidence `:subject` is a stable graph identity, not an ordinary provenance string. Rust canonical
source adaptation parses it as a stable ID before constructing the evidence record. Selfhost previously
classified whitespace-only subject as an empty required field (`code 4`), which diverged from the stable
ID invalidation boundary (`code 2`).

## Decision

- Do not classify `subject` through the ordinary required-field empty-string chain.
- Validate its wire shape before subject-kind and registry checks; malformed or whitespace-only values
  return invalid stable ID code `2` and preserve field/value `subject`.
- Keep ordinary whitespace-only rejection (`code 4`) for execution/provenance fields such as `runner`.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  whitespace-subject fixture variables and contract were absent from the inner smoke.
- Differential RED: Rust source oracle accepted the fixture only as
  `SourceGraphError::EdgeId(StableIdError::InvalidWireFormat { value: "  " })`; selfhost returned
  `["0", "4", "subject"]` before the fix.
- Rust oracle: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_whitespace_only_evidence_subject_as_invalid_id`
  passed and asserts the invalid stable-ID boundary.
- Rust-host selfhost oracle:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_evidence_registry::validation::test_e2e_selfhost_source_evidence_rejects_whitespace_only_subject_as_invalid_id -- --exact --nocapture`
  passed after wire-shape validation was added.
- The full selfhost evidence validation group (16 tests), native source-file provenance smoke, and
  docs/format checks are required gates under the fake Lima/provenance harness.

## Boundary and follow-up

This closes the whitespace-only evidence subject wire contract only. It does not prove current-source
packaged stage0 execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep
`EC-M2-02` and M2/M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
