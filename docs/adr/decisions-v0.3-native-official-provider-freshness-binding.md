# ADR: v0.3 official provider identity freshness and artifact binding preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/native-official-release-local.sh` provider identity preflight
- Related: [`decisions-v0.3-provider-identity-verification-clock.md`](decisions-v0.3-provider-identity-verification-clock.md), [`decisions-v0.3-native-official-provider-source-commit-binding.md`](decisions-v0.3-native-official-provider-source-commit-binding.md)

## Context

The official release gate required both provider snapshots and checked each of
the four identity files for schema, snapshot digests, and current source commit.
However, the caller-provided verification clock was not propagated to that
gate, and the App.Cli identity preflight did not pass its sibling
`program.native` bytes to the verifier. A future identity time or a changed
program could therefore reach release/package/smoke orchestration before the
offline identity verifier saw the relevant boundary.

## Decision

When provider snapshots are supplied, the gate accepts an optional explicit
`NATIVE_OFFICIAL_REVIEW_VERIFICATION_NOW` UTC clock and forwards it to the
identity verifier for all four target/stage0 identity inputs. For identity files
under an App.Cli artifact directory, the sibling `program.native` is also
passed as `--artifact`; stage0 identity inputs remain source/snapshot-bound
without assuming a compiler payload name. A verification clock without both
provider snapshots is rejected before downstream work.

Any freshness, source, snapshot, schema, or artifact mismatch fails before
release, stage0 packaging, fetch, source smoke, or Lima invocation.

## Evidence

- RED: the fake two-target harness accepted `now=2026-08-15T00:00:00Z` with
  verification clock `2026-08-14T23:59:59Z` and reached the fake release.
- RED: changing the App.Cli `program.native` bytes without changing its
  identity was not checked by the official gate.
- GREEN: the same harness rejects the future clock with
  `identity now is after verification now`, rejects the artifact mismatch with
  `artifact_digest mismatch`, and keeps the downstream invocation log
  unchanged. The valid two-target snapshot gate remains green.
- No live provider network, stage regeneration, full build, or Linux VM replay
  was used.

## Boundary and follow-up

This is an official-gate offline closure of provider-input, identity freshness,
source binding, and App.Cli artifact binding. It does not implement live
provider/auth acquisition, signature semantic verification, current-source
Linux runtime, or Mac/Linux packaged and rollback bytes parity. Those remain
`[~]` in `TODO.md` and the v0.3 planning document. The current checkout has no
matching manifest/expected replay lock, and Lima/QEMU/replayd are owned by
another session, so heavy replay remains deferred.
