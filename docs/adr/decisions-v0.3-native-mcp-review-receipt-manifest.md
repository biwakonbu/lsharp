# ADR: v0.3 native MCP review verification receipt manifest projection

## Status

Verified partial slice (2026-08-02). This decision fixes receipt projection
from `lsharp_validate` into an emitted manifest review only.

## Context

The previous receipt handoff connected Rust's verified result to the native
MCP report. The same request can also emit a manifest, whose `reviews[]`
entry is an existing release/evidence surface. Leaving that entry at only
`verification_state: verified` would allow the manifest to lose the
attestation digest, trust-store digest, provider/key binding, or verification
clock carried by the receipt.

## Decision

- When an explicit `review_verification_receipt` is supplied with
  `include_manifest`, the emitted manifest must contain exactly one review
  whose `namespace/key` is derived from the receipt `review_id`.
- That review must retain `verification_state: verified` and a closed
  `verification_receipt` object exactly equal to the validated receipt.
- A missing, ambiguous, or changed manifest projection is an MCP error after
  manifest postflight validation. Receipt-free manifests keep their existing
  projection and do not gain implicit verification data.
- The report projection and manifest projection use the same Rust receipt
  fixture; this connects the existing surface without adding another
  cryptographic verifier or changing provider snapshot semantics.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  manifest_projection` showed receipt-bearing manifest calls succeeding even
  when the manifest review omitted or changed the receipt.
- GREEN: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  explicit_verification_receipt` passed valid report+manifest projection,
  missing projection, changed projection, and schema cases.
- Rust receipt/signature/wire/trust-store focused suites and the existing
  Rust-host selfhost attestation/source-attestation exact tests passed with
  the shared receipt fixture.
- Linux VM replay, stage regeneration, and full build were not started:
  current-source manifest/expected lock was absent and the running
  Lima/QEMU/replayd resources are owned by another session.

## Remaining boundary

Native cryptographic verification, live provider/auth acquisition,
current-source Linux runtime, and Mac/Linux packaged/rollback bytes parity
remain `[~]` in TODO/planning.
