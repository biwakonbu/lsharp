# ADR: v0.2 native validation empty evidence source commit

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-evidence-graph.md`

## Context

Evidence provenance needs a non-empty source commit so a result remains tied to the exact source that
produced it. The Rust canonical `EvidenceGraph` rejects an empty source commit during registration, and
selfhost `Evidence` exposes that boundary as source error code `4` with wire field `source-commit`, but
the native source-file smoke did not exercise a source fixture.

## Decision

- Add a source fixture whose evidence record contains `:source-commit ""`.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:4`, empty stdout, and no manifest file.
- Keep the failure before report/manifest emission; do not infer source provenance from the current
  checkout, artifact digest, runner, or timestamp.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  empty-source-commit fixture variables and contract were absent from the inner smoke.
- Rust oracle: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_empty_required_execution_source_commit`
  passed and asserts `EvidenceValidationError::EmptyField { field: "source_commit" }`.
- Rust-host selfhost oracle:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_evidence_registry::validation::test_e2e_selfhost_source_evidence_rejects_empty_source_commit -- --exact --nocapture`
  passed with source error code `4` and wire field `source-commit`.
- Native source-file provenance smoke, runner tests, docs audit, and `git diff --check` passed under the
  fake Lima/provenance harness.

## Boundary and follow-up

This is a required-field source/native contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02` and
M2/M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
