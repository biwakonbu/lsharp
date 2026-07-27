# ADR: v0.2 native validation blank review provenance rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-invalid-review.md`

## Context

Review provenance must contain a non-blank digest. A whitespace-only value is not usable
provenance and must be rejected before review registry/report generation. The native smoke already
covered an unknown visibility value with code `8`, but not the separate non-blank digest check.

## Decision

- Add `:review "review:checkout/blank-digest" "   " "redacted"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:8`, empty stdout, and no manifest file.
- Keep blank digest rejection on the same invalid-review code as unknown visibility, distinct from
  invalid review IDs and duplicate reviews.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the blank-digest fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
