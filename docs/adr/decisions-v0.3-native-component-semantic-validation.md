# ADR: v0.3 native component semantic validation boundary

## Status

Verified partial slice (2026-08-02). Native component packaging now requires
the produced component to pass the explicit `wasm-tools validate` boundary
before atomic promotion.

## Context

The component postflight already rejected missing, symlinked, non-regular,
empty, and non-Wasm outputs. Those checks establish byte shape but cannot
prove that the component is structurally valid WebAssembly. Promoting a
byte-shaped but semantically invalid component would make a later runtime or
consumer own a failure that belongs at packaging time.

## Decision

- Run `wasm-tools validate <temporary-component>` after `component new` and
  the byte-shape check, but before `os.replace`.
- A non-zero validator result, including its stderr and exit status, fails
  closed. The requested existing output remains unchanged and the temporary
  component is cleaned up.
- Keep `wasm-tools` as an explicit external tool boundary. Do not call cargo,
  rustc, host `lsharp`, or a Rust fallback from the native helper.
- This slice proves semantic validation only. It does not claim component
  instantiation, source/ftable/import runtime parity, or Mac/Linux runtime
  evidence.

## Evidence

- RED: the fake `wasm-tools` produced a byte-shaped component that reported a
  semantic validation failure; the helper previously promoted it and did not
  invoke `validate`.
- GREEN: the same fake fixture records `component new` followed by
  `validate`, rejects the semantic failure before promotion, preserves the
  sentinel output, removes the temporary component, and keeps the existing
  child-failure, invalid-byte, warning, explicit-tool, and atomic-replace
  cases green.
- Focused native component harness, Python syntax, shell syntax, docs audit,
  and diff checks are the commit gates. No Linux replay, stage regeneration,
  or full build is used because the current-source manifest/expected replay
  lock is unavailable and the Lima/QEMU/replayd resources are owned by
  another session.

## Remaining boundary

Component instantiation, source/ftable/import parity, standalone runtime,
current-source Mac/Linux stage0 runtime, live provider/auth acquisition, and
Mac/Linux packaged/rollback bytes parity remain `[~]` under EC-M3-04 / EC-M3-05
and M3-04-N1 / M3-05-N9.
