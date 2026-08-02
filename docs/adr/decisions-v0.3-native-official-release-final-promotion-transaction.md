# ADR: Official native release final promotion transaction

## Status

Accepted as a verified offline/fake-harness partial slice for EC-M3-04 / EC-M3-05
(2026-08-02). Current-source Mac/Linux runtime, packaged/rollback byte parity,
and crash/power-loss filesystem durability remain open.

## Context

The official two-target release gate already stages App.Cli archives, stage0
archives, and `checksums.txt` until both target runtime/evidence postflights pass.
Its final promotion previously moved each staged regular file directly into
`DIST_DIR`. A failure after one move could therefore leave a mixed release: some
new managed files, some old managed files, and no deterministic restoration
boundary.

## Decision

- Treat the top-level regular files in the task-owned release staging directory
  as the managed publication set.
- Before each replacement, copy an existing managed regular destination into a
  task-owned transaction backup. Reject a managed destination that is a symlink
  or non-regular entry.
- Promote staged files one at a time, but on any promotion failure restore every
  moved destination from its backup and remove newly created managed files.
  Unrelated files in `DIST_DIR` are never included in the rollback set.
- Keep the deterministic `LSHARP_NATIVE_RELEASE_PROMOTION_FAIL_AFTER` injection
  test-only; normal release invocation defaults to no failpoint and exposes no
  production API for it. Remove the transaction directory on both success and
  rollback.

This is a bounded publication transaction. It does not claim power-loss atomicity
or durable `fsync` ordering, and it does not remove stale managed files absent
from a later staged set.

## Evidence and remaining boundary

The same fake two-target fixture passes the normal publication path and injects a
failure after the first final move. It verifies non-zero failure, restoration of
all pre-existing managed archives/checksums, preservation of an unrelated
sentinel, and no final-promotion transaction residue:

```text
bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh
bash scripts/ci/test-native-official-release-snapshots.sh
git diff --check
```

This is offline/fake orchestrator evidence only. It does not prove real
Mac/Linux current-source execution, packaged/rollback bytes parity, power-loss
filesystem durability, live provider/auth, or full native producer parity. The
current-source manifest and expected replay lock do not match this HEAD, and
another session owns the Lima/QEMU/replayd resources, so Linux replay, stage
regeneration, and full build were not run.
