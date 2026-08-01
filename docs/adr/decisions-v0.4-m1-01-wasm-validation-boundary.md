# ADR: v0.4 M1-01 explicit Wasm validation before runtime

## Status

Accepted for the semantic fixture producer boundary (2026-08-01,
implementation commit `a20fae09`). This ADR does not complete V4-M1-01 or the
Mac/Linux native artifact and runtime gates.

## Context

Both fixture producers checked that compilation created a regular artifact and
then passed it directly to Wasmtime. A regular file could still contain
invalid bytes. A fake or broken runner could therefore produce a report with
runtime output without proving that the artifact was a valid Wasm module.

## Decision

- Require callers to provide an absolute executable `--wasm-tools` path for
  both Rust-oracle and native-stage0 report producers.
- Run `wasm-tools validate <artifact>` after compilation and before runtime.
- On validation failure, return an explicit error, do not write the report, and
  do not start Wasmtime.
- Keep the Wasm validator as an explicit caller-owned boundary; no PATH search,
  host compiler, fallback, or network discovery is added.

## Evidence

- RED tests use a validator that rejects `not-wasm` bytes and assert that the
  report is absent and the runtime marker is untouched.
- Rust producer suite: 13 tests.
- Native producer suite: 13 tests.
- Diff suite: 7 tests.

## Consequences

Observed artifact/runtime evidence now has a real Wasm validation gate in both
producer paths. Existing pending, invalid-diagnostic, source-copy, and
fallback boundaries remain unchanged. Actual target-specific native runtime
parity remains pending.
