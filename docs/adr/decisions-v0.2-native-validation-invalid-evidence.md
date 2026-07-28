# ADR: v0.2 native validation invalid evidence enum rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-malformed-evidence.md`

## Context

The source adapter maps evidence `method`, `outcome`, `independence`, and `subject` to typed
canonical values. The Rust oracle already rejects an unknown enum value with a span-bearing
`InvalidEvidenceField` diagnostic, but the native source-file smoke did not cover this boundary.

## Decision

- Add an evidence fixture whose `:method` is `"not-a-method"` while all required fields are present.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:8`, empty stdout, and no manifest file.
- Keep invalid evidence-field rejection at source-adapter code `8`, distinct from malformed parser
  arity code `1` and evidence registry closure code `6`.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the invalid-evidence fixture variables and contract were absent from the inner smoke.
- GREEN: Rust oracle test `source_adapter_reports_invalid_evidence_enum_with_directive_span` passed,
  and the native source-file provenance test returned `Linux native stage0 source-file provenance tests: OK`
  under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a typed source-adapter/native source-file smoke contract only. It does not prove current-source
packaged stage0 execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion.
Keep `EC-M2-02`, `EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
