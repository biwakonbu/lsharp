# ADR: v0.2 native validation invalid evidence enum matrix

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-invalid-evidence.md`

## Context

The first native typed-field slice covered an unknown evidence `method`. The remaining source adapter
enum boundaries—`outcome`, `independence`, and evidence `subject` kind—also had Rust oracle coverage
but no native source-file smoke contract.

## Decision

- Add separate fixtures for an unknown `:outcome`, unknown `:independence`, and an evidence ID used as
  `:subject`.
- Each native `validate --source <fixture> --format json --emit-manifest <path>` invocation returns
  exit `1`, stderr `source validation error:8`, empty stdout, and no manifest file.
- Keep all typed evidence-field failures on source-adapter code `8`; parser arity remains code `1`.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  three matrix fixture variables and contracts were absent from the inner smoke.
- GREEN: Rust oracle test `source_adapter_reports_invalid_evidence_enum_with_directive_span` passed,
  and the native source-file provenance test returned `Linux native stage0 source-file provenance tests: OK`
  under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This matrix is a typed source-adapter/native source-file smoke contract only. It does not prove
current-source packaged stage0 execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback
exclusion. Keep `EC-M2-02`, `EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers it.
