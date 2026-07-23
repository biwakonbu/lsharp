# ADR: WasmGC `print-string` runner stdout sink

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: WasmGC core module runner for `--backend=wasmgc --target=web-wasm`

## Context

Stage 2h made it possible to read a packed `StringBytes` reference in a Wasmtime host callback, but no
public execution path installed that callback. Instantiating the synthetic import alone did not define
stdout ownership, sink failure behavior, or the boundary between WasmGC core execution and the existing
WASI Preview1/Preview2 runners.

## Decision

Add `lsharp_wasm::wasmgc_runner` with an explicit core-module runner contract.

- `run_wasm_wasmgc_with_stdout_sink` enables WasmGC, accepts only `env.print-string`, rejects every other
  import without a WASI fallback, and invokes exported `main: () -> i64`.
- One `print-string` invocation produces one immutable byte slice. The sink must consume that entire slice
  and return `Ok(())`; an `Err` is propagated as a Wasm error without retry or silent truncation.
- The i64 main result must fit i32 and is returned as the exit code. `run_wasm_wasmgc_capture` collects the
  same sink calls and decodes UTF-8; invalid UTF-8 is an explicit error. `run_wasm_wasmgc` additionally
  requires exit code 0, matching the existing convenience runner shape.
- This API runs a WasmGC core module only. It does not pretend to provide WASI fd_write, Component Model
  adapters, Unicode code-point semantics, native/selfhost runtime parity, or supported-target evidence.

## Evidence

- `wasm_gc_runner_connects_print_string_to_stdout_sink` executes a direct WasmGC module, observes UTF-8
  bytes `[195, 169]`, and preserves exit code `7`.
- `wasm_gc_runner_propagates_stdout_sink_failure` fixes the no-retry error boundary.
- `wasm_gc_runner_rejects_non_print_string_import_without_wasi_fallback` fixes the explicit external
  boundary.
- `test_compile_file_wasmgc_backend_runs_with_public_runner_stdout_sink` covers source parse/type/lower,
  WasmGC codegen, public runner instantiation, host callback, UTF-8 capture, and exit code.

## Consequences

- The WasmGC `print-string` path now has a runnable core-module boundary with deterministic sink/error
  semantics.
- The `std::io::Write` integration, including partial-write retries and flush errors, is recorded as the
  separate Stage 2j adapter decision. WASI/component output ownership remains outside this core runner.
- WASI/component output integration and the aggregate `LEGACY-EXEC-01` parity target remain active tasks;
  this verified slice is not a claim of full public WasmGC execution support.
