# ADR: v0.2 native validation invalid review rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-duplicate-review.md`

## Context

Review metadata has a strict visibility domain (`public` or `redacted`). An unknown value such
as `private` must fail before review registry/report generation rather than being accepted as
opaque provenance. Rust source adapter returns `InvalidReviewField` / stable code `8`; native
source-file smoke covered duplicate and missing review but not invalid metadata.

## Decision

- Add `:review "review:checkout/invalid" "sha256:review-provenance" "private"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:8`, empty stdout, and no manifest file.
- Keep invalid review field rejection distinct from duplicate review (`code 7`), missing review
  (`code 10`), and report-producing validation statuses.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the invalid-review fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers same fixture.
