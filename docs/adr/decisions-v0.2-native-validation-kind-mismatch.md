# ADR: v0.2 native validation node kind mismatch rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-01`、`EC-M3-02`、`docs/adr/decisions-v0.2-native-validation-invalid-id.md`

## Context

Source metadata carries both a form kind and an explicit stable ID. A node form such as
`:claim "intent:checkout/wrong-kind" ...` has valid wire syntax but violates the typed node
contract. It must fail before graph/report generation instead of being silently reclassified as an
intent. The Rust source adapter already exposes `KindMismatch`; native source smoke lacked this
distinct diagnostic boundary.

## Decision

- Add the `:claim "intent:checkout/wrong-kind"` node fixture.
- `validate --source <fixture> --format json --emit-manifest <path>` returns exit `1`, stderr
  `source validation error:3`, empty stdout, and no manifest file.
- Keep node kind mismatch distinct from malformed arity (`code 1`), invalid wire ID (`code 2`),
  and missing node (`code 5`).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the kind-mismatch fixture and required contract were absent from the inner smoke.
- GREEN: the focused command returned `Linux native stage0 source-file provenance tests: OK` under
  the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-03`
and the M3 aggregate `[~]` until actual stage0 replay covers the same fixture.
