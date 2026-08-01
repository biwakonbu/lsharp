# ADR: Rollback archive checksum coverage

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh`, `scripts/ci/test-release-smoke-provider-snapshots.sh`
- Related: `M3-05-N2`, `M3-05-N7`, `EC-M3-05`, `decisions-v0.3-release-smoke-checksum-path.md`

## Context

Rollback release smoke verified every line present in `checksums.txt`, but did not require checksum entries for
the payloads it later consumed. A rollback archive could omit the launcher checksum while retaining a valid
launcher, manifest identity, and all other listed checksums.

## Decision

Require rollback `checksums.txt` to cover the smoke-critical payload set: `README.md`, `LICENSE`, `lsharp`,
`lsharp-lsp`, `lsharp.component.wasm`, and `manifest.json`. This extends checksum evidence coverage without
changing checksum target path containment, archive entry, manifest, provider identity, or atomic-install rules.

## Evidence

- RED: the provider snapshot harness removed only the rollback `lsharp` checksum entry; the previous smoke accepted
  the archive and executed the valid launcher.
- GREEN: the same archive now fails with `checksums.txt missing required entry: lsharp`; valid rollback/provider and
  official release snapshot harnesses remain green.
- Current-source Linux replay was not started. Reproduce the blocker with `git rev-parse --verify HEAD` and
  `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`; no matching
  current-source manifest or expected hostgen lock was available at audit time.

## Boundary

This closes only rollback package checksum coverage for the local release smoke payload set. It does not prove live
provider/auth acquisition, current-source Linux runtime, packaged bytes parity across targets, rollback runtime parity,
or persistent-I/O recovery. The related v0.3 items remain `[~]` in `TODO.md`.
