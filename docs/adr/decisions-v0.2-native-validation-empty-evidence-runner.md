# ADR: v0.2 native validation empty evidence runner

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`, `crates/lsharp-types/src/validation_source/source_evidence.rs`, `crates/lsharp-types/tests/validation_source/evidence.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Evidence execution identity needs a non-empty runner. The Rust canonical `EvidenceGraph` rejects an
empty runner during registration, and selfhost `Evidence` exposes that boundary as source error code
`4`, but the native source-file smoke did not exercise a source fixture.
The selfhost source adapter checks required execution/provenance fields before stable-ID shape.
Rust source adapter previously parsed the evidence ID first, so an invalid ID combined with an empty
runner returned the wire-format error instead of the required-field error.

## Decision

- Add a source fixture whose evidence record contains `:runner ""`.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:4`, empty stdout, and no manifest file.
- Validate required execution/provenance string fields before parsing the evidence ID. For
  `:evidence "evidence:checkout"` with `:runner ""`, return
  `EvidenceValidationError::EmptyField { field: "runner" }` and source error code `4`.
- Keep the failure before report/manifest emission; do not synthesize a runner from the function name,
  generator, timestamp, or another provenance field.

## Evidence

- RED: the Rust source test failed because an invalid evidence ID was parsed before the empty runner;
  selfhost and native tests lacked the combined-precedence fixture.
- Rust oracle: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_empty_required_execution_runner`
  passed and asserts `EvidenceValidationError::EmptyField { field: "runner" }`.
- Rust source adapter precedence test passed with the invalid-ID/empty-runner fixture.
- Rust-host selfhost oracle:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_intent_source_adapter::test_e2e_selfhost_source_evidence_reports_empty_runner_before_invalid_id -- --nocapture`
  passed with source error code `4` and directive span `10..20`.
- Native source-file provenance smoke, runner tests, docs audit, and `git diff --check` passed under the
  fake Lima/provenance harness.

## Boundary and follow-up

This closes the evidence required-field precedence across Rust source adapter, selfhost direct consumer,
and native source-file smoke. It does not prove current-source packaged stage0 execution, Mac/Linux
artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02` and M2/M3 aggregate `[~]`.
