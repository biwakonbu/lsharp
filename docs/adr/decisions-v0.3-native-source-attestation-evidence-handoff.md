# ADR: v0.3 native source attestation evidence handoff

## Status

Verified partial slice (2026-08-02). This decision connects the existing
selfhost source-attestation report to the native source-file evidence writer.

## Context

Rust and selfhost already produce the same source-attestation projection,
including canonical bytes, state, and span. The source-file smoke evidence
writer copied command outputs and artifacts but did not preserve the
`review_attestations` output in its evidence manifest. That made the source
producer result disappear at the evidence boundary.

## Decision

- The source smoke cleanup passes the explicit
  `validation-attestation-json.stdout` report to the evidence writer.
- When that input is supplied, it must be a non-empty regular JSON file whose
  object contains a `review_attestations` array. The writer copies that array
  as `review_attestations` in the evidence `manifest.json`.
- A missing, empty, malformed, non-object, or missing-array report fails before
  creating the evidence directory. No attestation is synthesized when the
  input is omitted.
- The writer preserves the producer output as a handoff; it does not
  reimplement Rust attestation parsing or canonical-byte semantics.

## Evidence

- RED: `bash scripts/ci/test-native-selfhost-source-file-smoke-evidence.sh`
  rejected the not-yet-supported `--review-attestation-report` wiring.
- GREEN: the same harness preserved the exact `review_attestations` array and
  rejected a missing report without creating evidence. `bash -n` and
  `python3 -m py_compile scripts/ci/write-native-source-smoke-evidence.py`
  also passed.
- Existing Rust source adapter and selfhost producer focused evidence covers
  the shared fixture's named fields, canonical bytes, state, and span. No Rust
  producer code changed in this slice.
- Linux source smoke, stage regeneration, and full build remain unstarted:
  current-source manifest/expected lock is unavailable and Lima/QEMU/replayd
  are owned by another session.

## Remaining boundary

Native source smoke runtime, live provider/auth acquisition, current-source
Linux runtime, and Mac/Linux packaged provenance/rollback bytes parity remain
`[~]` in TODO/planning.
