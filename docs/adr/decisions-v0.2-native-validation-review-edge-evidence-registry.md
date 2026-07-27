# ADR: v0.2 native validation review-edge evidence registry rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-registry.md`

## Context

Review edges may consume intent, claim, or evidence subjects. Evidence subjects must be registered
before either `evaluates` or `invalidates` is accepted. The native smoke already rejected an
unregistered evidence `supports` edge, but the review-edge consumers were not covered for both
relations.

## Decision

- Add one `evaluates` fixture and one `invalidates` fixture targeting
  `evidence:checkout/missing` without an evidence registry entry.
- Both `validate --source <fixture> --format json --emit-manifest <path>` invocations return exit `1`,
  stderr `source validation error:6`, empty stdout, and no manifest file.
- Keep this registry-required error distinct from missing review (`code 10`) and subject-kind mismatch
  (`code 9`).

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the two review-edge evidence fixtures and contracts were absent from the inner smoke.
- GREEN: focused command returned `Linux native stage0 source-file provenance tests: OK` under
  fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixtures.
