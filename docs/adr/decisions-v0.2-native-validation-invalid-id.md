# ADR: v0.2 native validation invalid edge ID rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-01`、`EC-M3-02`、`docs/adr/decisions-v0.2-native-validation-malformed-edge.md`

## Context

Typed source edges require a wire prefix and namespace/key shape before endpoint existence or kind
checks can be trusted. An endpoint such as `claim:checkout` lacks the `/` separator and must not be
treated as an orphan, an unknown graph, or a report-producing validation result. The Rust source
adapter already returns its malformed-ID diagnostic, while native source-file smoke had no stable
invalid-ID case.

## Decision

- Add `:motivates "intent:checkout/safe-cancel" "claim:checkout"` as an invalid-ID fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:2`, empty stdout, and no manifest file.
- Keep malformed ID rejection separate from malformed arity (`code 1`), kind mismatch, and missing
  node (`code 5`) so callers can classify input failures deterministically.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the invalid-ID fixture and required contract were absent from the inner smoke.
- GREEN: the focused command returned `Linux native stage0 source-file provenance tests: OK` under
  the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-03`
and the M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
