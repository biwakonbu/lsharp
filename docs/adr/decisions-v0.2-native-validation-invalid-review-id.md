# ADR: v0.2 native validation invalid review ID rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-invalid-id.md`

## Context

Review metadata uses the same `kind:namespace/key` stable-ID shape as other typed nodes. A review
ID such as `review:checkout` lacks the required key segment and must fail before review registry or
report generation. Edge endpoint validation already covered code `2`, but review-form ID parsing did
not have a native source-file fixture.

## Decision

- Add `:review "review:checkout" "sha256:review-provenance" "redacted"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:2`, empty stdout, and no manifest file.
- Keep invalid review ID rejection distinct from invalid review metadata (`code 8`) and duplicate
  review IDs (`code 7`).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the invalid-review-ID fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
