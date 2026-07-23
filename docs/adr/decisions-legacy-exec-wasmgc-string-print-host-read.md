# ADR: WasmGC `print-string` host-side packed read

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: Wasmtime host callback for `--backend=wasmgc --target=web-wasm`

## Context

Stage 2g fixed `env.print-string` as a concrete GC import, but its host stub ignored the reference.
Treating the reference as an i64 pointer would silently reintroduce the linear-memory ABI and could read
arbitrary memory. The host boundary needs an explicit type check and a bounded byte extraction contract
before a stdout or WASI adapter is connected.

## Decision

Add `lsharp_wasm::wasmgc_host::create_print_string_import` with the following contract.

- The supplied `FuncType` must have exactly one nullable concrete array reference parameter, whose element
  storage is packed `i8`, and no results. Other signatures are rejected before creating the host function.
- The callback reads the parameter as `Val::AnyRef`, rejects null and non-array values, checks the concrete
  array element storage, reads `ArrayRef::len` and each element with `ArrayRef::get`, and passes immutable
  unsigned bytes to a caller-provided sink.
- Null, downcast/type mismatch, length/get failure, out-of-range byte, and sink failure become explicit
  Wasmtime host errors/traps. No i64 pointer or linear-memory access is introduced.
- This helper is an engine-level host boundary only. Runner stdout, WASI fd_write, component adapter,
  native/selfhost runtime, and Unicode code-point semantics remain separate tasks.

## Evidence

- `wasm_gc_host_print_string_reads_packed_bytes` executes a direct WasmGC module and captures UTF-8
  `[195, 169]` through the host sink.
- `wasm_gc_host_print_string_rejects_null_reference_at_runtime` confirms null references trap with the
  explicit host error, and `wasm_gc_host_print_string_rejects_non_packed_import_signature` rejects an
  `i32` array signature before instantiation.
- `test_compile_file_wasmgc_backend_reads_print_string_with_host_import` covers source parse/type/lower,
  WasmGC codegen, Wasmtime instantiate, host callback, and captured bytes in one pipeline test.

## Consequences

- The first host read of a WasmGC String is bounded by the GC array length and cannot fall back to a raw
  pointer ABI.
- The public WasmGC runner still does not install this helper or define stdout/partial-write semantics;
  WASI/component/native/selfhost parity and Unicode code-point output remain active under
  `LEGACY-EXEC-01`.
