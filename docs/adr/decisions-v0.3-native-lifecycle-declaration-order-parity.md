# ADR: v0.3 native lifecycle declaration-order parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` lifecycle snapshot reducer
- Related: [`decisions-v0.3-selfhost-review-lifecycle.md`](decisions-v0.3-selfhost-review-lifecycle.md), [`decisions-v0.3-review-wire.md`](decisions-v0.3-review-wire.md)

## Context

Rust `ReviewLifecycleRegistry::from_events` and the selfhost lifecycle reducer
normalize provider events by `(review_id, sequence)` before applying the
append-only transition rules. The native release identity verifier instead
reduced the raw JSON/JSONL declaration order. A valid `active` sequence 1
followed by `revoked` sequence 2 was therefore rejected when the provider
returned the two records in reverse order.

This is a producer declaration-order parity boundary. It does not add another
sequence gap, rollback, duplicate, effective-at, clock, schema, or provider
snapshot rule.

## Decision

Before lifecycle semantic validation, the native verifier sorts object records
by the canonical `(review_id, sequence)` key, matching the Rust reducer. It
keeps the existing fail-closed diagnostics for malformed records and preserves
the legacy `sequence rollback` diagnostic when a declaration-order rollback
would otherwise become an impossible terminal-first history after normalization.

## Evidence

- RED: the new native fixture with `revoked` sequence 2 before `active` sequence
  1 failed with `review lifecycle initial state` before the implementation.
- GREEN: the same fixture is accepted by
  `python3 scripts/ci/test-native-release-identity.py`; the full native identity
  suite passes 33 tests, including the existing rollback, gap, duplicate,
  transition, and effective-at cases.
- Rust oracle: `cargo test -q -p lsharp-types --test review_wire
  wire_lifecycle_declaration_order_does_not_change_reduced_state -- --exact`
  passes.
- Selfhost parity: `cargo test -q -p lsharp-wasm --test e2e
  e2e::selfhost_evidence_registry::lifecycle::selfhost_lifecycle_reducer_orders_events_and_rejects_invalid_transitions -- --exact`
  passes one test.
- No provider network, stage regeneration, full build, or Linux VM replay was
  used.

## Boundary

This closes only the native producer's declaration-order parity with the Rust
and selfhost reducers. Live provider/auth acquisition, cryptographic
attestation verification, current-source Linux runtime, and Mac/Linux
packaged/rollback artifact parity remain unverified. The related EC-M3 items
remain `[~]` in `TODO.md` and the v0.3 planning document.
