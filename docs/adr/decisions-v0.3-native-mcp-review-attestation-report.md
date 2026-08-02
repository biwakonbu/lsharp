# ADR: v0.3 native MCP review-attestation report projection

## Status

Verified partial slice (2026-08-02). This decision connects the Rust source
attestation report wire shape to the native MCP validation output boundary.

## Context

Rust's canonical `lsharp_validate` source route projects each
`:review-attestation` into `review_attestations[]` with named fields,
canonical bytes, verification state, and source span. The native MCP shim
accepted the surrounding report fields but treated this valid projection as an
unknown field, so the source/native MCP report contract could not be compared.

## Decision

- `review_attestations` is an optional native validation report field with the
  same closed 14-field wire shape as the Rust projection.
- The native postflight validates review ID, non-empty identity strings,
  `ed25519`, unpadded base64url signature shape, canonical UTC timestamps,
  positive u64 sequence, explicit verification state, byte-valued canonical
  bytes, and non-negative source span positions.
- This is a wire/projection boundary only. It does not perform cryptographic
  verification or infer provider/lifecycle trust, and it does not alter the
  existing explicit receipt or report/manifest projection contracts.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  review_attestation_report` rejected a valid Rust-shaped report as
  `unknown field: review_attestations`.
- GREEN: the same focused tests accept the complete fixture and reject missing,
  extra, invalid-state, out-of-range-byte, and invalid-span variants.
- Native MCP 93 tests, Rust
  `test_validate_tool_projects_source_attestation_as_unverified`, source-file
  evidence harness, and the official two-target fake release harness passed.

## Remaining boundary

Signature verification, live provider/auth acquisition, current-source Linux
runtime, and Mac/Linux packaged/rollback bytes parity remain `[~]` in
TODO/planning. Linux replay, stage regeneration, and full build were not
started because current-head manifest/expected replay lock evidence was
absent and Lima/QEMU/replayd were owned by another session.
