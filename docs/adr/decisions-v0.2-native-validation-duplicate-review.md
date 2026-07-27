# ADR: v0.2 native validation duplicate review rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-missing-review.md`

## Context

Review provenance is a registry keyed by stable review ID. Two records with the same ID but
different digests cannot both be authoritative; accepting the second one would make downstream
`evaluates`/`invalidates` edges ambiguous. The Rust source adapter returns `DuplicateReview` (stable
code `7`), while native source-file smoke did not require this uniqueness failure.

## Decision

- Add two `:review` forms for `review:checkout/duplicate` with different provenance digests.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:7`, empty stdout, and no manifest file.
- Keep duplicate review rejection distinct from invalid review fields, missing review, and report
  producing validation statuses.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the duplicate-review fixture and contract were absent from the inner smoke.
- GREEN: the focused command returned `Linux native stage0 source-file provenance tests: OK` under
  the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and the M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
