# ADR: v0.3 provider identity verification clock freshness

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: offline `verify-native-release-identity.py --verification-now`
- Related: [`decisions-v0.3-provider-lifecycle-future-effective-at.md`](decisions-v0.3-provider-lifecycle-future-effective-at.md)

## Context

The release identity carried a strict UTC `now`, and lifecycle events were
checked against that identity time. Nothing bound the identity time itself to
the caller's observation clock. A provider identity could therefore be
accepted as fresh even when its claimed `now` was later than the time at which
the caller performed verification.

## Decision

The offline verifier accepts an explicit `--verification-now` UTC clock. When
present, it validates the clock shape and rejects an identity whose `now` is
later than that clock with `identity now is after verification now`. The
caller-provided clock is explicit and deterministic; the verifier does not
silently read wall-clock time or acquire provider data.

## Evidence

- RED: `python3 scripts/ci/test-native-release-identity.py -k test_rejects_identity_now_after_explicit_verification_clock`
  failed because the verifier had no `--verification-now` boundary.
- GREEN: the focused test rejected identity `now=2026-08-15T00:00:00Z` for
  verification clock `2026-08-14T23:59:59Z`, and the full identity suite passed
  all 32 tests.
- No network, stage regeneration, full build, or Linux VM replay was used.

## Boundary and follow-up

This is an explicit offline identity-freshness boundary, separate from
lifecycle `effective_at` ordering and provider snapshot digest/regular-file
checks. It does not prove live provider/auth acquisition, signature semantic
verification, current-source Linux runtime, or Mac/Linux packaged and rollback
bytes parity. Those remain `[~]` in `TODO.md` and the v0.3 planning document.
The current HEAD has no matching manifest/expected replay lock, and another
session owns the Lima/QEMU/replayd processes.
