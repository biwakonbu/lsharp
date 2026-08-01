# ADR: v0.3 review attestation sequence lower bound

## Status

Accepted for the Rust canonical model and Rust-host selfhost source adapter
slice (2026-08-01). Native stage0, Linux x86_64, and packaged provenance
parity remain part of the EC-M3-04/05 completion boundary.

## Context

The review provenance schema declares `sequence` as an integer with minimum
`1`, and lifecycle events already rejected `0`. The attestation constructor
and selfhost source validator nevertheless accepted `sequence: 0`, allowing a
wire/source attestation to disagree with the lifecycle and JSON Schema
contract.

## Decision

- Reject `sequence == 0` in `ReviewAttestation::new` with the typed
  `AttestationError::InvalidSequence` error.
- Preserve the stable source validation code `8` and directive span when the
  Rust source adapter or selfhost source adapter rejects the field.
- Apply the same `sequence >= 1` lower bound to the selfhost validation path;
  values above the unsigned wire range remain rejected by the existing parser.
- Keep native stage0 and packaged target evidence as separate follow-up gates;
  Rust-host success is not promoted to native completion.

## Evidence

- Rust attestation model: 5 tests passed.
- Rust source adapter: 7 tests passed, including zero-sequence span/error.
- Rust wire boundary: 6 tests passed, including zero-sequence rejection.
- Rust-host selfhost E2E:
  `e2e::selfhost_intent_source_adapter::test_e2e_selfhost_source_adapter_rejects_invalid_attestation_fields`
  passed.

## Consequences

Attestation, lifecycle, schema, Rust source, and selfhost source now share the
same lower-bound contract. Native source-file smoke and current-source
packaged artifact evidence remain `[~]` and must be run by the owner of the
native gate.
