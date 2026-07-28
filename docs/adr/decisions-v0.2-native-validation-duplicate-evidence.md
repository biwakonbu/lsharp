# ADR: v0.2 native validation duplicate evidence ID rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-selfhost-evidence-registry.md`

## Context

Evidence registry identity is stable and must not be overwritten by a later declaration. Rust source
adapter tests retain both declaration spans for a duplicate evidence ID, and the selfhost registry
consumer rejects the second registration, but native source-file smoke had no duplicate evidence fixture.

## Decision

- Add two complete evidence records with the same `evidence:checkout/duplicate` ID and distinct runner /
  artifact values.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:3`, empty stdout, and no manifest file.
- Do not merge or last-write-wins duplicate evidence; preserve the first/duplicate span boundary in
  the typed source adapter and keep duplicate-ID code `3` distinct from invalid-field code `8`.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  duplicate-evidence fixture variables and contract were absent from the inner smoke.
- GREEN: Rust oracle test `source_adapter_reports_duplicate_evidence_with_both_source_spans` passed,
  and the native source-file provenance test returned `Linux native stage0 source-file provenance tests: OK`
  under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a duplicate source-registry/native source-file smoke contract only. It does not prove
current-source packaged stage0 execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback
exclusion. Keep `EC-M2-02`, `EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers it.
