# ADR: v0.3 official fetched stage0 identity binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/native-official-release-local.sh`, `scripts/ci/test-native-official-release-snapshots.sh`
- Related: `M3-04-N1`, `M3-05-N2`, `M3-05-N8`, `M3-05-N9`

## Context

The official two-target postflight now compares the fetched stage0 payload digest with source-smoke evidence, but
the evidence fields for target, source commit, and the manifest digest were still not checked against the fetched
`manifest.json` and current checkout. A producer could emit a payload-bound evidence record while substituting a
different manifest identity.

## Decision

After each target source smoke, `native-official-release-local.sh` reads the fetched
`${SMOKE_ROOT}/stage0-${target}/manifest.json`, rejects invalid or symlinked manifests, and recomputes its SHA-256.
It then requires one exact identity tuple in the target evidence: target, current `SOURCE_COMMIT`,
`stage0_manifest_sha256`, and the already-verified `stage0_payload_sha256`. The fetched manifest target and source
commit must also match the target and current checkout before the evidence projection is accepted.

This extends the existing offline source-smoke provenance binding. It does not change the stage0 manifest schema,
reimplement fetch checksum/archive validation, or perform native cryptographic/provider verification.

## Evidence

- RED fixture: the fake two-target source smoke can emit a wrong Linux payload or manifest digest while the previous
  postflight only checked review attestations and would not reject that identity substitution.
- GREEN: `bash scripts/ci/test-native-official-release-snapshots.sh` accepts matching Mac/Linux target, source,
  manifest, and payload identity, and rejects both payload and manifest-digest mismatches with
  `source smoke evidence stage0 identity mismatch`.
- The gate is fake/offline. Current-source manifest/expected replay lock is absent for the current checkout and
  Lima/QEMU/replayd are owned by another session, so no Linux replay, stage regeneration, or full build was started.

## Boundary and follow-up

This closes fetched stage0 manifest/source identity binding to official target evidence. It does not prove actual
Mac/Linux runtime execution, live provider/auth acquisition, native cryptographic verification, packaged bytes parity,
or rollback runtime parity. The related M3 items remain `[~]`.
