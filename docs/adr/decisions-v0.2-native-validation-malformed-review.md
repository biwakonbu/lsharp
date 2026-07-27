# ADR: v0.2 native validation malformed review rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-malformed-edge.md`

## Context

Review metadata has a fixed three-field payload: stable ID, provenance digest, and visibility. A
review form missing visibility must fail as malformed input before review registry/report generation.
The native smoke covered malformed ordinary edges but not malformed review metadata arity.

## Decision

- Add `:review "review:checkout/malformed" "sha256:review-provenance"` fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:1`, empty stdout, and no manifest file.
- Keep malformed review arity distinct from invalid review fields (`code 8`) and malformed edge
  payloads using the same parser-level code `1`.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the malformed-review fixture and contract were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
