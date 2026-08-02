# ADR: v0.3 external review receipt trust identity handoff

## Status

Verified partial for the offline provider/crypto boundary. This does not add
cryptographic signature verification to native MCP.

## Context

`verify-native-review-verification-receipt.py` already validates the closed
receipt shape and emits the Rust-compatible canonical receipt bytes. The
receipt names a provider, key, and algorithm, while the existing trust-store
validator independently selects active keys. Without a binding step, a validly
shaped receipt could name an inactive or unrelated key before an external
verifier consumed it.

## Decision

The receipt verifier accepts an optional explicit
`--trust-store TRUST_STORE_JSON` input. It requires that the trust store be a
regular non-symlink file, reuses `verify-native-review-trust-store.py` for its
closed shape and active-key ambiguity checks, and requires exactly one active
identity matching the receipt's `(provider, key_id, algorithm)`. Invalid,
inactive, unrelated, or ambiguous identities fail before the handoff succeeds.

Native MCP remains an offline receipt consumer and does not verify Ed25519
signatures or acquire provider/auth data. The existing digest-only snapshot
guard and receipt/lifecycle boundary are unchanged; no receipt fields or
schema are added.

## Evidence

- RED: the new matching, mismatch, and invalid trust-store fixtures failed
  because the verifier only accepted `RECEIPT_JSON` and had no trust identity
  admission.
- GREEN: the same fixtures pass with the explicit trust-store option; the
  mismatch and invalid cases fail closed with no successful handoff.
- Focused command:
  `PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=scripts/ci python3 scripts/ci/test-native-review-verification-receipt.py`
- No network, provider acquisition, native signature verification, Linux
  replay, stage regeneration, or full build was run.

## Remaining boundary

Current-source Mac/Linux runtime, full Rust/native producer parity,
packaged/rollback parity, live provider/auth acquisition, and real Ed25519
signature verification remain unverified. The related EC-M3 and M3-04-N1 /
M3-05-N9 items stay `[~]`.
