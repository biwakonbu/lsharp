# ADR: v0.2 native validation invalidation missing review rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-missing-review.md`

## Context

When a review registry is present, both `evaluates` and `invalidates` edges must consume a
registered review. The native source-file smoke already rejected an unregistered `evaluates`
consumer with stable code `10`, but the equivalent `invalidates` path was not covered.

## Decision

- Add a registered review and an `:invalidates` edge targeting `review:checkout/missing`.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:10`, empty stdout, and no manifest file.
- Preserve the same missing-review code for both edge relations and keep it distinct from subject-kind
  mismatch (`code 9`) and invalid review metadata (`code 8`).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the invalidation-missing-review fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
