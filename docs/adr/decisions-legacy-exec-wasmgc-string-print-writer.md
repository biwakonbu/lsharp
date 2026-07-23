# ADR: WasmGC `print-string` `std::io::Write` adapter

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: Rust host adapter for the WasmGC core runner

## Context

Stage 2i exposed a sink callback, but a caller that owns a Rust `Write` implementation still needed to
define how partial writes, zero-length writes, and final flush errors affect Wasm execution. Passing only
the first `write` result would silently truncate UTF-8 chunks.

## Decision

Add `lsharp_wasm::wasmgc_runner::run_wasm_wasmgc_to_writer`.

- The writer is held behind `Arc<Mutex<_>>` so the Wasmtime host callback can own it for the synchronous
  execution lifetime. Each `print-string` chunk is passed to `Write::write_all`, which retries partial
  writes until the complete chunk is consumed.
- `WriteZero` and any write error become an explicit sink error and stop execution. No retry policy beyond
  `write_all`, byte loss, or chunk reordering is allowed. After successful Wasm execution, `flush` is called;
  a flush error is returned to the caller.
- This adapter returns the exported i32 exit code and does not install WASI imports or convert the core
  module into a Component Model artifact.

## Evidence

- `wasm_gc_runner_write_adapter_retries_partial_writes_until_chunk_is_consumed` uses a writer that accepts
  one byte per call and verifies the complete UTF-8 chunk.
- `wasm_gc_runner_write_adapter_propagates_write_error` and
  `wasm_gc_runner_write_adapter_rejects_write_zero` fix the write failure boundaries.
- `wasm_gc_runner_write_adapter_propagates_flush_error_after_execution` fixes post-execution flush error
  propagation.

## Consequences

- Host applications can connect WasmGC core stdout to any owned `std::io::Write` implementation without
  losing partial chunks.
- WASI fd_write / Component Model adapter, Unicode code-point semantics, native/selfhost runtime parity,
  and supported target evidence remain active under `LEGACY-EXEC-01`.
