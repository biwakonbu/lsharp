# ADR: v0.2 native validation empty review ID rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-invalid-review-id.md`

## Context

Review metadata requires a stable ID. An empty review ID is a missing required field, not merely a
malformed wire shape, and must fail before review registry/report generation. The native smoke now
covered malformed review IDs and invalid provenance, but not the empty-ID branch.

## Decision

- Add `:review "" "sha256:review-provenance" "redacted"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:8`, empty stdout, and no manifest file.
- Keep empty review ID rejection distinct from malformed review IDs (`code 2`) and blank digest or
  unknown visibility invalid-review fields.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the empty-review-ID fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
