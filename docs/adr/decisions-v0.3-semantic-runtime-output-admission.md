# ADR: v0.3 semantic runtime stdout/stderr admission

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: Rust-oracle/native-stage0 semantic fixture report producers
- Related: [`decisions-v0.3-semantic-runtime-exit-admission.md`](decisions-v0.3-semantic-runtime-exit-admission.md)、[`decisions-v0.3-semantic-runtime-artifact-binding.md`](decisions-v0.3-semantic-runtime-artifact-binding.md)

## Context

The previous runtime admission boundary compared only the canonical expected exit
code. A valid fixture could therefore finish with exit `0` while emitting different
stdout or stderr, and the producer would still publish an observed report. The later
Rust/native diff can identify that mismatch, but a producer report is itself an
observable evidence input and must not describe an unexpected runtime as accepted.

## Decision

- For a valid fixture, Rust-oracle and native-stage0 producers compare observed
  decoded stdout and stderr exactly with `expected.runtime.stdout` and
  `expected.runtime.stderr` when those canonical values are present.
- Any mismatch fails closed before report publication with a stream-specific
  `runtime stdout/stderr ... does not match expected ...` diagnostic.
- The existing runtime exit admission, invalid-fixture `not-run` behavior, report
  schema, artifact digest binding, and caller-owned cleanup semantics remain unchanged.

## Evidence

- RED: fake Wasmtime returned exit `0` with an unexpected stdout or stderr for the
  same `valid/syntax-basic` fixture; both producers published a report.
- GREEN: Rust 17 tests and native 18 tests cover stdout mismatch, stderr mismatch,
  no-report failure, and the existing valid, input snapshot, invalid, source mutation,
  and batch cleanup behavior.
- This is offline/fake evidence only; no real target runtime or packaged artifact was
  used.

## Boundary

This is a verified partial runtime output admission slice. Real component execution,
current-source Mac/Linux producer parity, packaged/rollback parity, provider/auth,
and full target runtime evidence remain unverified. EC-M3-04 / EC-M3-05 and
M3-04-N1 / M3-05-N9 remain `[~]`.
