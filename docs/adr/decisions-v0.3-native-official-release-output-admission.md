# ADR: Official native release output admission after target evidence

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Current-source
Mac/Linux runtime, packaged bytes parity, and full release transactionality
remain open.

## Context

The official two-target release gate packages App.Cli and stage0 archives before
running the fetched stage0 source smoke and cross-target evidence postflight.
If the later Mac/Linux evidence projection failed, the archives had already
been written to the caller's final `DIST_DIR`. That made a failed target gate
look like a partially published release and could leave stale output alongside
the failure.

## Decision

Keep all package and checksum output under the existing task-owned
`SMOKE_ROOT/release-dist` staging directory. The official gate uses that staged
directory for the local archive URL and all package/release smoke operations.
Only after both target fetch/runtime postflights and the cross-target evidence
projection pass are staged files moved into the caller's final `DIST_DIR`.

The bounded contract is failure-before-promotion: a late target evidence
failure does not publish new archives and preserves pre-existing final output.
The per-file final promotion itself is not claimed to be a crash-consistent
filesystem transaction; that remains a separate durability/transactionality
boundary.

## Evidence

The existing fake two-target harness now injects a Linux-only shared-field
mismatch after both target package paths have run and asserts that existing
output remains while no new archive is published:

```text
bash scripts/ci/test-native-official-release-snapshots.sh
```

This is offline/fake orchestrator evidence. It does not prove current-source
stage0 execution on Mac or Linux, actual VM runtime behavior, or packaged and
rollback byte parity across targets. The current manifest/expected replay lock
is unavailable and another session owns the Lima/QEMU/replayd resources, so no
Linux replay, stage regeneration, or full build was run.

## Consequences

The final release directory is now an admission boundary for the completed
two-target evidence gate rather than the working directory for intermediate
archives. Existing release output is not deleted when a later source-smoke or
cross-target evidence check fails. EC-M3-04 / EC-M3-05 and M3-04-N1 / M3-05-N9
remain `[~]` until actual target runtime and packaged provenance evidence are
available.
