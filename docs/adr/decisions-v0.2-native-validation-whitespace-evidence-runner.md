# ADR: v0.2 native validation whitespace-only evidence runner

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Required evidence strings must contain a non-whitespace value. The Rust canonical
`EvidenceGraph` applies `trim().is_empty()`, while selfhost `Evidence` previously checked only
`string-length`, allowing a whitespace-only `:runner` to pass and produce a report.

## Decision

- Reject whitespace-only required evidence fields before report or manifest emission.
- Mirror the Rust policy with a selfhost `source-evidence-nonblank?` helper that recognizes space,
  tab, LF, and CR as whitespace.
- Preserve the existing `runner` field name, source error code `4`, exit `1`, empty stdout, and
  no-manifest fail-closed boundary.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  whitespace-runner fixture variables and contract were absent from the inner smoke.
- Differential RED: the Rust-host selfhost E2E initially returned `["1", "-9223372036854393736"]`
  for `:runner "  "` instead of `["0", "4", "runner"]`, proving a real parity gap.
- Rust oracle: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_whitespace_only_required_execution_runner`
  passed and asserts `EvidenceValidationError::EmptyField { field: "runner" }`.
- Rust-host selfhost oracle:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_evidence_registry::validation::test_e2e_selfhost_source_evidence_rejects_whitespace_only_runner -- --exact --nocapture`
  passed after the helper was added.
- Native source-file provenance smoke, runner tests, docs audit, and `git diff --check` are required
  gates under the fake Lima/provenance harness.

## Boundary and follow-up

This closes the whitespace semantics for the required evidence `runner` source contract only. It does
not prove current-source packaged stage0 execution, Mac/Linux artifact/runtime parity, manifest bytes,
or fallback exclusion. Keep `EC-M2-02` and M2/M3 aggregate `[~]` until actual stage0 replay covers the
same fixture.
