# ADR: v0.3 official source-smoke cross-target projection parity

## Status

Accepted for the offline official-gate contract slice (2026-08-02). This does
not complete current-source runtime or packaged/rollback parity.

## Context

The official two-target gate already validated each Mac and Linux source-smoke
manifest against its fetched stage0 and, when supplied, the explicit review
attestation report. That per-target check did not reject a target producer
adding a target-only field or changing the shared manifest projection while
remaining individually valid.

## Decision

After both target source-smoke postflights, the orchestrator compares the two
manifest projections. `target`, `stage0_manifest_sha256`, and
`stage0_payload_sha256` are target-specific. Every other key must be present in
both manifests with the same JSON value. The target values themselves remain
fixed to Mac Apple Silicon and Linux x86_64. A key-set or shared-value mismatch
fails closed with a stable diagnostic before the official gate reports success.

The fake two-target harness injects an otherwise valid Linux-only projection to
prove the negative boundary. The normal fixture proves that target-specific
stage0 digests may differ while the shared projection remains accepted. This
slice is an offline shell/orchestrator contract; it is not Rust canonical
verification, a live provider/authentication result, or real target runtime
evidence.

## Evidence and remaining boundary

- `bash scripts/ci/test-native-official-release-snapshots.sh` — normal
  two-target projection and Linux-only-field fail-closed fixture.
- `bash -n scripts/ci/native-official-release-local.sh
  scripts/ci/test-native-official-release-snapshots.sh` — shell syntax.

The current-source Mac/Linux runtime, packaged App.Cli bytes, rollback/Wasm
parity, live provider/auth acquisition, and Rust/native producer parity remain
unverified. Their TODO entries stay `[~]`. The current source manifest and
expected replay lock are not available for this HEAD, and another session owns
the running Lima/QEMU/replayd resources, so Linux replay, stage regeneration,
and full build were intentionally not run.
