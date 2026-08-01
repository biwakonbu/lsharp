# ADR: Provider snapshot regular-file boundary

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py`, `scripts/ci/test-native-release-identity.py`
- Related: `M3-05-N2`, `M3-05-N7`, `EC-M3-05`

## Context

The offline release identity verifier recalculated trust-store and lifecycle snapshot digests from caller-provided
paths. Its byte reader followed symlinks, so a provider/auth input could reference bytes outside the intended
snapshot file while still producing a matching digest. This boundary was independent of URL validation, archive
entry validation, manifest binding, and atomic installation.

## Decision

Before reading provider snapshot bytes, require each path to be a regular non-symlink file using `lstat`. Reject
directories, other special files, and symlinks with an explicit identity input error. The shared verifier keeps this
rule consistent for native packaging, release smoke, and stage0 package callers.

## Evidence

- RED: `python3 scripts/ci/test-native-release-identity.py` added a trust-store and lifecycle symlink fixture whose
  targets had the expected bytes; the previous verifier accepted it.
- GREEN: the same harness now rejects the fixture with a regular non-symlink diagnostic. The provider snapshot,
  official release snapshot, and release identity focused harnesses all pass.
- Current-source Linux replay was not started. Reproduce the blocker with `git rev-parse --verify HEAD` and
  `find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'`; no manifest matching
  the current source and no expected hostgen lock were available at audit time.

## Boundary

This verifies only offline provider snapshot path provenance. It does not prove live provider/auth acquisition,
current-source Linux runtime, packaged bytes parity across targets, rollback runtime parity, or provider semantics.
The related v0.3 items remain `[~]` in `TODO.md`.
