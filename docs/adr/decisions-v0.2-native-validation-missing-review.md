# ADR: v0.2 native validation missing review rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-registry.md`

## Context

When a source review registry is present, `evaluates` and `invalidates` edges must reference a
registered review identity. Otherwise a typo can be accepted as an external review and the report
can claim evidence provenance that was never registered. The Rust source adapter exposes
`MissingReview` (stable code `10`); native source-file smoke covered valid review edges but did not
exercise this registry closure failure.

## Decision

- Add one registered review and an `:evaluates "review:checkout/missing"` edge in the same fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:10`, empty stdout, and no manifest file.
- Keep missing-review rejection distinct from malformed IDs, kind mismatch, evidence registry
  required, and missing node errors.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the missing-review fixture and contract were absent from the inner smoke.
- GREEN: the focused command returned `Linux native stage0 source-file provenance tests: OK` under
  the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and the M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
