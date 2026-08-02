# ADR: v0.3 semantic runtime artifact identity binding

## Status

Verified partial slice (2026-08-02). Rust-oracle and native-stage0 semantic
fixture reports bind each observed runtime result to the exact Wasm artifact
digest that was validated and executed.

## Context

The semantic fixture producers already reported an artifact digest and runtime
stdout/stderr separately. That shape did not make the source-to-artifact-to-
runtime relationship explicit to a downstream evidence consumer: a runtime
result could be copied beside a different observed artifact without the report
itself exposing the mismatch. `wasm-tools validate` and the opt-in Wasmtime
postflight are necessary checks, but neither is an evidence identity contract.

## Decision

- Add `runtime.artifact_sha256` to observed Rust-oracle and native-stage0
  reports; it is the digest of the same artifact passed to `wasmtime run`.
- Pending and not-run runtime entries carry a null `artifact_sha256`.
- The report diff rejects an observed runtime without an observed artifact and
  rejects any runtime digest that differs from its report's artifact digest.
- Rust/native differential comparison includes the runtime artifact digest,
  while expected fixture stdout/stderr and exit code remain the semantic
  oracle. No source, ftable, import, or target-runtime claim is inferred from
  this field.

## Evidence

- RED: the diff fixture first supplied a runtime digest, and the existing report
  schema rejected the extra field. A negative fixture then demonstrated that a
  runtime digest different from the artifact must fail closed.
- GREEN: Rust-oracle, native-stage0, diff, target evidence audit, and two-target
  aggregate fixtures pass with the exact artifact/runtime binding. Existing
  pending evidence remains pending rather than being promoted.
- The focused batch is offline/fake-harness evidence; it does not execute a
  current-source Mac/Linux stage0 or prove source/ftable/import parity.

## Remaining boundary

This slice does not prove current-source target runtime, source/ftable/import
producer parity, native stage0 regeneration, live provider/auth acquisition,
or Mac/Linux packaged and rollback bytes parity. Those remain `[~]` under
EC-M3-04 / EC-M3-05 and M3-04-N1 / M3-05-N9.
