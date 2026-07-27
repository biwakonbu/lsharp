# ADR: v0.2 native validation malformed review edge rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-malformed-review.md`

## Context

Review edges have a fixed two-endpoint payload. Both `evaluates` and `invalidates` must reject a
form missing one endpoint before review graph/report generation. The native smoke covered malformed
ordinary edges and malformed review metadata but not the two review-edge relations.

## Decision

- Add an `:evaluates "review:checkout/registered"` fixture with a missing subject.
- Add an `:invalidates "change:checkout/api-v2"` fixture with a missing subject.
- Both `validate --source <fixture> --format json --emit-manifest <path>` invocations return exit `1`,
  stderr `source validation error:1`, empty stdout, and no manifest file.
- Keep malformed review-edge arity on parser-level code `1`, distinct from missing review (`code 10`)
  and subject-kind mismatch (`code 9`).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the two malformed review-edge fixtures and contracts were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixtures.
