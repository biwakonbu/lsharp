# ADR: Native component runtime-to-package artifact identity

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Real current-source
Mac/Linux component runtime and full producer parity remain open.

## Context

The native component helper already has an explicit opt-in `wasmtime run`
postflight after `wasm-tools validate`, but the runtime is an external process.
Without an identity check, an external runtime that changes the temporary
component and exits successfully could cause bytes different from the
instantiated component to be atomically promoted as the packaged output.

This is distinct from the previous static source/ftable/import projection: the
contract here is the lifecycle of the actual temporary component through
validation, instantiation/run, and package promotion.

## Decision

`scripts/native-selfhost-component.py` records the SHA-256 of the validated
temporary component immediately before the optional runtime invocation. When
`--wasmtime PATH` is supplied, a successful `wasmtime run` is accepted only if
the temporary component remains byte-identical. A runtime mutation therefore
fails closed before `os.replace`, preserving the existing output and cleaning
the temporary component. The digest is taken after semantic validation so the
bytes promoted without runtime are also the bytes that passed validation.

No runtime receipt/report schema is added and no implicit runtime is enabled;
the existing explicit `--wasmtime` boundary remains the only path that runs a
component.

## Evidence

The existing fake native → `wasm-tools component new` → `validate` → optional
`wasmtime run` harness now covers the successful run and a deterministic
runtime-mutation failure:

```text
python3 scripts/ci/test-native-selfhost-component.py
```

The mutation case exits non-zero, retains the pre-existing packaged output,
and leaves no temporary component. The fake harness proves ordering and
fail-closed behavior only; it is not evidence of a real current-source
component instantiation on either supported target.

## Consequences

The packaged component cannot silently diverge from the bytes that the explicit
runtime postflight executed. Real Wasmtime/component semantics, Rust/native
producer parity, current-source Mac/Linux runtime, and packaged/rollback
parity remain `[~]` until current artifacts and target evidence are available.
