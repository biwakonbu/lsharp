# ADR: v0.2 native validation malformed review extra-arity rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-malformed-review.md`

## Context

Review metadata has exactly three wire fields: stable ID, opaque provenance digest, and visibility.
The native smoke already rejected a missing visibility field, but an extra field could otherwise be
accepted as a following body expression unless the parser-level arity boundary is covered explicitly.

## Decision

- Add a `:review "review:checkout/malformed" "sha256:review-provenance" "redacted" "extra"` fixture.
- The Rust syntax oracle rejects the fixture with `LS0101`.
- Native `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:1`, empty stdout, and no manifest file.
- Keep review metadata arity rejection on parser-level code `1`, distinct from invalid review metadata
  (`code 8`) and malformed review-edge arity (also `code 1`, but a different form).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the extra review fixture variables and contract were absent from the inner smoke.
- GREEN: focused Rust syntax test and the native source-file provenance test passed under the fake
  Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a parser/native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
