# ADR: Native component runtime evidence receipt

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Real current-source
Mac/Linux component execution, Rust/native producer parity, and packaged/rollback
parity remain open.

## Context

The component helper has an explicit `--wasmtime` postflight, but its stdout,
stderr, and exit status were not preserved as evidence tied to the source and
validated component. Treating a successful external process as implicit proof
would make the runtime/evidence boundary unobservable and could hide a receipt
write failure before package promotion.

This is distinct from static source/ftable/import projection and from the
component-byte mutation guard: it records what the explicit runtime observed.

## Decision

Add an explicit `--runtime-evidence PATH` option to
`scripts/native-selfhost-component.py`. The option is accepted only together
with an explicit `--wasmtime PATH`. After semantic validation, the helper runs
the external runtime, captures its exit code and UTF-8-lossy stdout/stderr, and
atomically writes a separate JSON receipt containing the command, absolute
source path, source SHA-256, and validated temporary component SHA-256. Receipt
creation must succeed before the temporary component is promoted; a receipt
failure therefore fails closed and preserves the existing output.

The receipt is a new evidence sidecar, not an extension of the existing report
schema. Runtime mutation remains rejected before receipt creation and promotion.
The option is explicit, so no runtime or external provider is invoked by
default.

## Evidence

The fake native → `wasm-tools component new` → `validate` → explicit
`wasmtime run` harness verifies the positive contract, including runtime output,
source/artifact binding, atomic receipt creation, packaged bytes, and temporary
cleanup:

```text
python3 scripts/ci/test-native-selfhost-component.py
```

The 16 focused tests are offline evidence only. They do not prove real
component instantiation, current-source Mac/Linux runtime behavior, or target
packaged/rollback parity.

## Consequences

The runtime/evidence handoff is deterministic and fail-closed without changing
the existing report schema or enabling an implicit runtime. Real target
artifacts, Rust/native producer parity, current-source runtime, packaged and
rollback evidence, and provider/auth remain `[~]` until their required evidence
is available.
