# ADR: v0.3 native MCP compile/run external runtime boundary

## Status

Verified partial slice (2026-08-01). Native MCP `lsharp_compile_run` now
compiles through the native program and executes the resulting Wasm through an
explicit external `wasmtime` boundary. Supported-target stage0 and packaged
runtime parity remain separate work.

## Context

The Rust MCP server's compile/run tool accepts either inline source or a file,
produces a temporary `Main.wasm`, and returns formatted source plus runtime
stdout. A native MCP implementation must preserve that observable contract
without invoking `cargo`, `rustc`, a host `lsharp`, or an implicit runtime. The
shared Linux replay is not required for this boundary and must not be started.

## Decision

- Add `lsharp_compile_run` to the native MCP tool list with the same exclusive
  `source`/`file` input alternatives as the Rust MCP schema.
- Copy source/file content into a task-owned temporary `Main.ls`, invoke only
  `program.native compile Main.ls -o Main.wasm`, require a non-empty artifact,
  and remove all temporary files after the request.
- Resolve the Wasm runtime from `LSHARP_WASMTIME` when set, otherwise from
  `PATH` as `wasmtime`. A missing or non-executable runtime is an MCP error;
  cargo, rustc, host `lsharp`, and network/provider helpers are never fallback
  paths.
- Return `{ok, formatted, stdout, exit_code}` only after a zero-status runtime.
  Native compile failures, missing/empty artifacts, runtime launch failures,
  and non-zero runtime status fail closed as MCP tool errors and never reuse a
  stale artifact.

## Evidence

- `scripts/ci/native_selfhost_mcp_compile_tests.py` and
  `scripts/ci/test-native-selfhost-mcp.py` cover source/file routes, exclusive
  and unknown argument rejection, explicit runtime resolution, compile and
  runtime failure, missing artifact, no host fallback, and temporary artifact
  cleanup. Five compile/run tests are included in the 52-test native MCP suite.
- The fake native compiler and fake `wasmtime` record the exact boundary and
  prove compiler stdout is not confused with runtime stdout.
- Focused MCP tests, native LSP relay tests, native selfhost runner tests,
  Python compilation, documentation audit, and diff checks pass without a
  Linux VM replay.

## Remaining boundary

This is a local external-runtime verified slice, not proof of Mac/Linux native
stage0, packaged artifact, or Wasm ABI parity. Package installation, provider
acquisition/authentication, and supported-target runtime evidence remain `[~]`
under `EC-M3-05` / M3-05-N9.
