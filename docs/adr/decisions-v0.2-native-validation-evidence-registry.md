# ADR: v0.2 native validation evidence registry closure

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-kind-mismatch.md`

## Context

`supports` and `contradicts` edges are meaningful only when their evidence record is registered.
Accepting an unregistered evidence ID would turn a registry closure error into a misleading graph
report. Rust `SourceGraphError::EvidenceRegistryRequired` already defines this boundary; the native
source-file smoke had only positive evidence coverage and did not require the diagnostic path.

## Decision

- Add a fixture that declares a claim and `:supports "evidence:checkout/missing"` without an evidence
  record.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:6`, empty stdout, and no manifest file.
- Keep registry-required rejection separate from malformed arity/ID, kind mismatch, and missing node
  diagnostics.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the registry-required fixture and contract were absent from the inner smoke.
- GREEN: the focused command returned `Linux native stage0 source-file provenance tests: OK` under
  the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and the M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
