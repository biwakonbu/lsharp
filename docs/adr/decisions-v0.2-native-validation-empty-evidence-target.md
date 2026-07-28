# ADR: v0.2 native validation empty evidence target

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Evidence execution identity needs a non-empty target so a result cannot be detached from the artifact
platform it claims to validate. The Rust canonical `EvidenceGraph` rejects an empty target during
registration, and selfhost `Evidence` exposes that boundary as source error code `4`, but the native
source-file smoke did not exercise a source fixture.

## Decision

- Add a source fixture whose evidence record contains `:target ""`.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:4`, empty stdout, and no manifest file.
- Keep the failure before report/manifest emission; do not infer the target from the host, runner, or
  source metadata.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  empty-target fixture variables and contract were absent from the inner smoke.
- Rust oracle: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_empty_required_execution_target`
  passed and asserts `EvidenceValidationError::EmptyField { field: "target" }`.
- Rust-host selfhost oracle:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_evidence_registry::validation::test_e2e_selfhost_source_evidence_rejects_empty_target -- --exact --nocapture`
  passed with source error code `4`.
- Native source-file provenance smoke, runner tests, docs audit, and `git diff --check` passed under the
  fake Lima/provenance harness.

## Boundary and follow-up

This is a required-field source/native contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02` and
M2/M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
