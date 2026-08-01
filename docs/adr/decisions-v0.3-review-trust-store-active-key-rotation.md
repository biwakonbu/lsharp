# ADR: v0.3 review trust-store active-key rotation

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-01` trust-store wire/parser and native provider preflight
- Related: [`decisions-v0.3-review-trust-store.md`](decisions-v0.3-review-trust-store.md)

## Context

The explicit trust store previously rejected only an exact `(provider, key_id,
algorithm)` duplicate. It could represent neither a retired key during key
rotation nor the invariant that a provider/algorithm has one current signing
key. A consumer could therefore choose a different key after a rotation, or
accept an ambiguous snapshot before signature verification was connected.

## Decision

- A trust key has an `active` boolean. Omitted v1 wire values remain
  backwards-compatible and mean `active: true`; rotation snapshots mark the
  old key `active: false` and the replacement `active: true`.
- `ReviewTrustStore::add_key` allows retired keys but rejects more than one
  active key for the same provider/algorithm. `active_key` exposes the
  deterministic selection, and attestation verification treats a retired key
  as `unverified`.
- The native provider preflight accepts the explicit `{"keys": [...]}`
  snapshot shape, applies the same default and identity rules, and rejects
  ambiguous active selection before a future semantic verifier consumes the
  snapshot. It does not claim to verify Ed25519 signatures.
- The schema/parser and native preflight remain separate from provider
  acquisition, signature verification, lifecycle reduction, and target
  runtime/package evidence.

## Evidence

- RED: the Rust rotation tests failed because `ReviewTrustKey` had no active
  state/selection API; the native tests failed because the preflight did not
  exist.
- GREEN: `cargo test -q -p lsharp-types --test review_trust_store
  --test review_signature --test review_wire --test validation_schema`,
  the two exact selfhost attestation/source focused tests,
  `python3 scripts/ci/test-native-review-trust-store.py` (2 passed), and
  `python3 -m py_compile` for both native scripts passed.
- The Rust retired-key signature test and native ambiguous-active fixture use
  the same provider/algorithm/key rotation semantics. No Linux VM replay,
  stage regeneration, or full build was started because the current-source
  manifest/expected lock and VM ownership were unavailable.

## Boundary

This is a verified partial trust-store selection/preflight slice. Native
cryptographic signature verification, live provider/auth acquisition,
current-source Linux runtime, and Mac/Linux packaged/rollback parity remain
`[~]` in TODO/planning.
