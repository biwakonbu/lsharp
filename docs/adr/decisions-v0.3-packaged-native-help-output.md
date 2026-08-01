# ADR: v0.3 packaged native-only help output boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh` native-only `--help` output
- Related: [`decisions-v0.3-packaged-lsp-version-output-parity.md`](decisions-v0.3-packaged-lsp-version-output-parity.md)

## Context

The native-only release smoke checked that `program.native --help` contained a
usage marker, but it did not inspect stderr. A packaged executable could
therefore leak a warning or diagnostic to stderr while still being accepted as
a healthy boot-surface result. This was an unverified packaged runtime output
boundary, separate from CLI version output and archive/rollback manifest or
checksum validation.

## Decision

`release-smoke.sh` captures native-only `--help` stdout and stderr separately.
The command must succeed, stdout must contain `Usage: lsharp`, and stderr must
be empty. Any stderr bytes fail closed with
`native-only App.Cli help must keep stderr empty` before the smoke is reported
as successful.

## Evidence

- RED: `bash scripts/ci/test-release-smoke-provider-snapshots.sh` accepted a
  checksum-valid fake native package whose `--help` wrote a warning to stderr.
- GREEN: the same fake package is rejected with the stable diagnostic; the
  valid native/rollback fixture continues to pass.
- The focused harness uses temporary offline archives. It does not invoke
  provider APIs, stage regeneration, a full build, or Linux VM replay.

## Boundary and follow-up

This verifies only the native-only packaged `--help` stdout/stderr contract.
It does not prove live provider/auth acquisition or semantic signature
verification, current-source Linux runtime, or Mac/Linux packaged and rollback
bytes parity. Those remain `[~]` in `TODO.md` and the v0.3 planning document.
The current checkout has no matching manifest/expected replay lock, and the
Lima/QEMU/replayd processes are owned by another session, so heavy replay stays
deferred.
