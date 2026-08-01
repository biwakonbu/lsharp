# ADR: Native-only rollback anchor kind parity

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh`, `scripts/ci/test-release-smoke-provider-snapshots.sh`
- Related: `M3-04-N1`, `M3-05-N2`, `EC-M3-05`, `decisions-v0.3-rollback-archive-manifest-payload-parity.md`

## Context

The native-only release smoke already bound the primary manifest to the supplied rollback archive by asset name
and archive SHA-256, and recursively validated the rollback archive's own kind, target, version, and source commit.
It did not validate the primary manifest's `rollback_anchor.kind`. A primary archive could therefore describe an
incorrect rollback kind while all payload, checksum, and nested archive checks passed.

## Decision

Require `manifest.rollback_anchor.kind` to equal `rollback compatibility` before accepting the primary native-only
archive. This binds the primary declaration to the rollback archive kind already required by the recursive smoke.
Existing payload-name, checksum, source identity, URL, and atomic-install contracts remain unchanged.

## Evidence

- RED: the provider snapshot harness changed only `rollback_anchor.kind`, recomputed the primary checksums, and used
  the valid rollback archive; the previous smoke accepted the mismatch.
- GREEN: the same fixture now rejects it with `rollback compatibility anchor kind mismatch`; the valid archive and
  existing provider identity failures remain covered.
- Current-source Linux replay was not started. Reproduce the blocker with `git rev-parse --verify HEAD` and
  `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`; no manifest matching
  the current source and no expected hostgen lock were available at audit time.

## Boundary

This closes only the local primary-to-rollback archive kind binding. It does not prove live provider/auth,
current-source Linux runtime, packaged bytes parity across targets, rollback runtime parity, or persistent-I/O
recovery. The related v0.3 items remain `[~]` in `TODO.md`.
