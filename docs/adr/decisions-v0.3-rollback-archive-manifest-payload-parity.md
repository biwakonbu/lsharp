# ADR: Rollback archive manifest payload parity

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh`, `scripts/ci/test-release-smoke-provider-snapshots.sh`
- Related: `M3-04-N1`, `M3-05-N2`, `EC-M3-05`

## Context

The rollback compatibility release smoke already checked the archive kind, target, version, source commit,
component Wasm magic, and checksums. It did not compare the rollback manifest's declared payload names with the
files consumed by the host-launcher smoke. A rollback archive could therefore carry valid checksums and matching
identity while declaring a different entry binary.

## Decision

Require the rollback manifest to declare the release assembler's fixed payload names before the rollback smoke
executes them:

- `entry_binary`: `lsharp`
- `lsp_binary`: `lsharp-lsp`
- `component`: `lsharp.component.wasm`

The existing checksum, target/version/source-commit, archive-entry, and atomic-install contracts remain unchanged.

## Evidence

- RED: the focused provider snapshot harness rebuilt a rollback archive with a mismatched `entry_binary`, then
  updated its checksums and the primary archive's rollback anchor; the previous smoke accepted it.
- GREEN: the same harness now rejects it with the explicit `entry_binary` mismatch diagnostic, while the valid
  primary/rollback fixture and existing provider snapshot failures remain covered.
- Focused shell syntax, docs audit, and diff checks are run with this slice. No Linux VM replay is included.

## Boundary

This verifies only local rollback package manifest-to-payload declaration parity. It does not prove live provider/auth,
current-source Linux runtime, packaged artifact provenance across targets, rollback byte parity, or persistent-I/O
recovery. The related v0.3 milestones remain `[~]` in `TODO.md`.
