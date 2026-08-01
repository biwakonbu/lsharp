# ADR: v0.3 review verification receipt handoff

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-01` Rust signature verification to native/provider handoff
- Related: [`decisions-v0.3-review-trust-store-active-key-rotation.md`](decisions-v0.3-review-trust-store-active-key-rotation.md)

## Context

Rust already owns the Ed25519 verification primitive, while the native
provider adapter intentionally keeps semantic verification external. Passing
only `review_id=verified` across that boundary loses which attestation bytes,
trust snapshot, and explicit clock were verified. Reimplementing Ed25519 in
the native Python adapter would create a second cryptographic implementation
and would not prove target runtime parity.

## Decision

- `ReviewVerificationReceipt::from_verified_signature` calls the Rust
  attestation verifier and refuses to create a receipt for an unknown/retired
  key or a signature error.
- A receipt binds `review_id`, `provider`, `key_id`, `algorithm`, the SHA-256
  digest of the attestation canonical bytes, the explicit trust-store digest,
  and the explicit UTC verification clock. Its state is always `verified`.
- Rust and native share a domain-separated, length-prefixed canonical byte
  order for the receipt. The native preflight validates the closed JSON shape,
  digest shape, real calendar date, and canonical bytes, but does not claim to
  perform cryptographic signature verification.
- Existing native provider snapshot paths remain the raw/unverified adapter;
  a receipt is an explicit external semantic-verifier handoff and does not
  change the existing `unverified` fallback or diagnostic semantics.

## Evidence

- RED: Rust receipt tests failed because the receipt module did not exist, and
  native receipt tests failed because the preflight did not exist.
- GREEN: `cargo test -q -p lsharp-types --test review_verification_receipt`
  (4 passed), `python3 scripts/ci/test-native-review-verification-receipt.py`
  (3 passed), and Python syntax compilation passed.
- The Rust fixture covers successful Ed25519 verification and unknown-key
  rejection. Rust/native canonical receipt fixtures match byte-for-byte,
  including the non-existent-date rejection. Linux VM replay, stage
  regeneration, and full build were not started because current-source
  manifest/expected-lock and VM ownership prerequisites were absent.

## Boundary

This is a verified partial verification-result handoff. Native cryptographic
verification, live provider/auth acquisition, current-source Linux runtime,
and Mac/Linux packaged/rollback parity remain `[~]` in TODO/planning.
