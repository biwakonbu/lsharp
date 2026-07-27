# ADR: v0.2 native validation review subject-kind rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-missing-review.md`

## Context

Review edges have typed subjects. `evaluates` may target an intent, claim, or evidence, while a
review ID is the consumer on the left side. A review ID used as the subject is structurally valid
as a wire ID but semantically invalid. The Rust source adapter reports `EdgeSubjectKindMismatch`
with stable code `9`; the native source-file smoke covered missing and invalid review registry cases
but not this subject-kind boundary.

## Decision

- Add a registered review and an `:evaluates` edge whose subject is that `review:` ID.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:9`, empty stdout, and no manifest file.
- Keep review subject-kind rejection distinct from invalid review metadata (`code 8`), duplicate
  review IDs (`code 7`), and missing review registry entries (`code 10`).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the review-subject-kind fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
