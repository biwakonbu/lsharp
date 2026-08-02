# ADR: v0.3 semantic two-target observation parity

## Status

Accepted as a verified offline/fake-harness partial slice (2026-08-02).
This does not complete current-source Mac/Linux runtime or packaged/rollback
parity.

## Context

The semantic evidence aggregate already re-audited both target indexes and
required the same fixture scope. It also recomputed Rust/native parity inside
each target. That was not enough to prevent both producers in one target from
agreeing on a target-specific source digest or semantic observation that
disagreed with the other target.

## Decision

- After both target indexes pass their own audit, compare target-independent
  observations for each fixture and each producer (`rust-oracle` and
  `native-stage0`).
- Require exact equality for `source_sha256`, diagnostics, and compiler
  `exit_code` across the Mac and Linux target reports.
- When both target runtimes are observed, require exact equality for runtime
  exit code, stdout, and stderr. A pending runtime remains pending rather than
  being promoted to parity; target-specific artifact bytes and artifact
  digests are intentionally not compared.
- Reject a mismatch before emitting a passing aggregate, with a stable
  `cross-target semantic parity mismatch` diagnostic. The report and aggregate
  JSON schemas are unchanged; this is an executable relation over existing
  evidence.

## Evidence and remaining boundary

- `python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py -v` — both
  target positive/pending paths and cross-target source observation mismatch.
- `bash scripts/ci/test-v4-m1-01-contracts.sh` — combined fixture, differential,
  producer, evidence, aggregate, docs, and whitespace focused batch.

The evidence is offline/fake-harness only. Current-source Mac/Linux runtime,
full native producer parity, packaged/rollback bytes, and live provider/auth
remain unverified and stay `[~]` in TODO/planning. The current-source manifest
and expected replay lock do not match this HEAD, and another session owns the
Lima/QEMU/replayd resources, so Linux replay, stage regeneration, and full
build were not run.
