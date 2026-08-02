# ADR: official two-target review-attestation report handoff

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-04` / `EC-M3-05`, official source-file smoke orchestration

## Context

The source-file evidence writer can project an explicitly supplied review-attestation report, but the official
Mac/Lima orchestrator previously invoked each target without a shared report input. That left a one-sided
propagation or copy-out mismatch indistinguishable from a successful source smoke. It also allowed future wiring to
silently create a report projection when the operator did not supply one.

## Decision

`native-official-release-local.sh` accepts the optional `NATIVE_OFFICIAL_REVIEW_ATTESTATION_REPORT` input.

1. When supplied, the report must be a non-empty regular non-symlink file, a JSON object, and contain a
   `review_attestations` list. It must be paired with `NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` so the handoff
   has an observable target output.
2. The same report path is passed once to the Mac source smoke and once to the Linux wrapper. The Linux wrapper
   copies that input into its VM work directory and passes the copied regular file to the guest writer.
3. After each target smoke, the orchestrator reads the target evidence manifest and exact-compares its
   `review_attestations` list with the explicit report. Missing, one-sided, copied, or changed values fail closed
   before the official gate reports success.
4. When no report is supplied, target evidence manifests must not contain `review_attestations`; no implicit report
   is generated.

This contract is limited to orchestrator handoff and target evidence projection. It does not claim live provider/auth,
signature semantics, current-source Linux runtime, or packaged target/rollback parity.

## Evidence

- RED: `bash scripts/ci/test-native-official-release-snapshots.sh` initially reached a fake target without an
  explicit report preflight or propagation/postflight mismatch boundary.
- GREEN: the same fake two-target harness verifies missing-report preflight with no invocation log change, exact
  Mac/Linux `review_attestations` projection, report-free no-implicit output, and Linux mismatch rejection.
- GREEN: `bash scripts/ci/test-native-selfhost-source-file-smoke-evidence.sh`
- GREEN: `bash -n scripts/ci/native-official-release-local.sh scripts/ci/native-selfhost-dev-source-file-smoke.sh scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh scripts/ci/test-native-official-release-snapshots.sh`
- GREEN: `python3 -m py_compile scripts/ci/write-native-source-smoke-evidence.py`
- GREEN: `bash scripts/audit_docs.sh` and `git diff --check`

The current-source Linux runtime gate remains blocked because a manifest/expected replay lock matching the current
checkout is unavailable and another session owns the Lima/QEMU/replay processes. No real VM replay or stage
regeneration was started for this slice.

## Consequences

The official fake gate now distinguishes explicit report handoff from report-free compatibility behavior and retains
target-specific evidence as an exact, auditable projection. The full M3-04-N1/M3-05-N9 runtime and packaged parity
requirements remain `[~]` in planning/TODO until current Mac/Linux runtime and artifact evidence are available.
