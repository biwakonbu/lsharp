# ADR: v0.3 semantic observation-state parity

## Status

Accepted as a verified offline/fake-harness partial slice (2026-08-02).
This does not complete current-source Mac/Linux runtime or packaged/rollback
parity.

## Context

The Rust-oracle/native-stage0 diff helper compared artifact and runtime
payloads only when both producers reported `observed`. If one producer
reported `observed` and the other reported `pending`, the helper classified
the boundary as pending. That could hide a producer implementation or
execution-state divergence and prevent the semantic producer parity gate from
reporting an observable mismatch.

## Decision

- Compare the artifact observation status and runtime observation status
  before comparing their payloads.
- Different statuses are an observable `mismatch` on
  `artifact.status` or `runtime.status`, and therefore return the existing
  non-zero mismatch result.
- Equal `pending` statuses retain the existing pending behavior. Equal
  `observed` statuses continue through the existing exact artifact/runtime
  comparisons; no report schema fields are added.

## Evidence

The same fake `valid/syntax-basic` fixture first demonstrated RED: one
producer was `observed` while the other remained `pending`, and the helper
returned an input error rather than a mismatch projection. After the status
comparison was added, the asymmetric artifact/runtime states are projected as
`mismatch`, while the existing all-pending and both-observed paths remain
GREEN:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v scripts/ci/test-semantic-fixture-diff.py
```

## Boundary and follow-up

This verifies only the offline relation between existing Rust/native semantic
report observation states. It is not evidence for current-source Mac/Linux
runtime execution, full native producer parity, packaged/rollback bytes,
live provider/auth acquisition, or a current-source manifest and expected
replay lock. Those boundaries remain `[~]` in the active planning and TODO
records. Another session owns Lima/QEMU/replayd, so Linux replay, stage
regeneration, and full build were not run.
