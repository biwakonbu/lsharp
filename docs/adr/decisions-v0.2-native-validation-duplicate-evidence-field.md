# ADR: v0.2 native validation duplicate evidence named field

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-duplicate-evidence.md`

## Context

An evidence record is a fixed set of named fields. Repeating a field must not silently overwrite the
first value or create an ambiguous source projection. The Rust syntax oracle rejects a repeated field
with `LS0101`, but the native source-file smoke did not cover this parser boundary.

## Decision

- Add a fixture that repeats `:subject` in one evidence record.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:1`, empty stdout, and no manifest file.
- Keep duplicate named-field rejection at the parser boundary; do not infer last-write-wins semantics
  or pass a partially populated evidence record to the graph registry.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  duplicate named-field fixture variables and contract were absent from the inner smoke.
- GREEN: Rust syntax oracle test `evidence_record_metadata_rejects_duplicate_named_field` passed, and
  the native source-file provenance test returned `Linux native stage0 source-file provenance tests: OK`
  under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a parser/native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02` and
M2/M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
