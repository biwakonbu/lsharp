# ADR: v0.4 M1-01 one-command contract gate

## Status

Accepted for local V4-M1-01 contract validation (2026-08-01). This is an
execution workflow improvement; it does not claim native stage0, Linux, or
two-target runtime completion.

## Context

The semantic fixture boundary has several short, independent Python contract
suites. Running each command manually between related edits adds orchestration
overhead and makes it easy to omit a schema, aggregate, or docs check. The
individual commands are still useful when diagnosing one failure, but the
normal validation path should be a single fail-fast invocation.

## Decision

- Add `scripts/ci/test-v4-m1-01-contracts.sh` as the batch entry point.
- Run matrix, diff, Rust/native producer, evidence schema/audit, aggregate
  schema/audit, producer-docs, repository docs, and whitespace checks in a
  deterministic order with `set -euo pipefail`.
- Keep the individual commands in the milestone plan for targeted diagnosis.
- Treat this gate as a local contract check only; it does not replace the
  native stage0, Wasm runtime, Linux x86_64, or two-target evidence gates.

## Evidence

- `scripts/ci/test-v4-m1-01-contract-runner.py` verifies executable mode,
  fail-fast shell options, and command ordering.
- `bash scripts/ci/test-v4-m1-01-contracts.sh` is the single command used for
  the grouped contract gate.

## Consequences

Related fixture edits can be validated once at the end of a slice while a
failed command remains easy to identify from its labeled section. Heavy Cargo,
native stage0, VM, and release gates remain explicit and are not started by
this fast local runner.
