# ADR: v0.3 native MCP review verification receipt projection

## Status

Verified partial slice (2026-08-02). This decision fixes the explicit receipt
handoff and `lsharp_validate` report projection only.

## Context

Rust now produces a verified review receipt after checking the Ed25519
signature against an explicit trust store. The native MCP adapter previously
had no way to receive that result: provider snapshots remained raw digest
inputs, and a `verified` report state without a native semantic verifier was
rejected. Passing only the state would lose the attestation, trust snapshot,
and verification-clock binding.

## Decision

- `lsharp_validate` accepts an explicit `review_verification_receipt` regular
  file and rejects a symlink, missing file, malformed JSON, or an invalid
  receipt before invoking native code.
- The validated receipt is forwarded as
  `--review-verification-receipt` to the native validate command.
- When that input is present, the native report must contain exactly one
  `review_verifications` item with the receipt's `review_id`, `verified` state,
  and byte-for-byte equal closed receipt object. A missing, ambiguous, or
  changed projection is an MCP error.
- Existing provider snapshot input without an explicit receipt remains the
  raw/unverified path; a verified state is still rejected there. This slice
  does not implement native cryptographic verification.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  explicit_verification_receipt` rejected the new input as an unknown
  argument.
- GREEN: `python3 scripts/ci/test-native-selfhost-mcp.py` passed the focused
  native MCP suite, including valid handoff, missing projection, changed
  projection, and invalid receipt/no-native-execution cases.
- Rust receipt, signature, wire, and trust-store suites passed together with
  the existing canonical receipt fixture; the native receipt validator suite
  also passed.
- Linux VM replay, stage regeneration, and full build were not started:
  current-source manifest/expected lock was absent and the running
  Lima/QEMU/replayd resources are owned by another session.

## Remaining boundary

Native cryptographic verification, live provider/auth acquisition,
current-source Linux runtime, and Mac/Linux packaged/rollback bytes parity
remain `[~]` in TODO/planning.
