# ADR: v0.3 native component runtime postflight boundary

## Status

Verified partial slice (2026-08-02). Component packaging supports an explicit
external runtime postflight without changing the default compile/build path.

## Context

`wasm-tools validate` proves structural WebAssembly validity, but it does not
prove that a packaged component can be instantiated and run by the configured
runtime. The native helper must not silently treat a runtime failure as a
successful packaged artifact, while ordinary compile/build must remain an
artifact-producing command unless the caller opts into runtime evidence.

## Decision

- Accept an optional `--wasmtime PATH` on the component packaging helper.
- When supplied, run `wasmtime run <temporary-component>` after byte-shape and
  `wasm-tools validate` checks, but before `os.replace`.
- A non-zero runtime result, including stderr and exit status, fails closed;
  the existing output remains unchanged and the temporary component is
  cleaned up.
- Keep runtime execution an explicit external boundary. The helper does not
  call cargo, rustc, host `lsharp`, or a fallback runtime, and default
  compile/build behavior does not start Wasmtime.

## Evidence

- RED: the new fake runtime contract requires the helper to invoke
  `wasmtime run` before promotion and to reject a deterministic runtime exit
  failure. The pre-contract helper had no `--wasmtime` option.
- GREEN: the fake component fixture records runtime invocation, accepts a
  successful postflight, rejects runtime exit `31`, preserves the sentinel
  output, and removes the temporary component. Existing default packaging,
  semantic validation, invalid artifact, child failure, explicit-tool, and
  atomic replacement cases remain green.
- This is offline/fake runtime evidence only. No current-source stage0
  artifact, Linux replay, or target runtime was available for a real replay.

## Remaining boundary

This does not prove real Wasmtime component instantiation on current-source
Mac/Linux stage0 artifacts, source/ftable/import parity, standalone runtime
coverage, live provider/auth acquisition, or Mac/Linux packaged/rollback
bytes parity. Those remain `[~]` under EC-M3-04 / EC-M3-05 and M3-04-N1 /
M3-05-N9.
