# ADR: WasmGC packed array の linear-memory output bridge

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC `print-string` の GC array を Component canonical output へ渡す guest bridge

## Context

Stage 2l で、Component Model の `list<u8>` が core module では exported linear memory 上の
`(ptr: i32, len: i32)` として lower される契約を固定した。WasmGC の
`PackedByteArray` は GC reference のため、そのまま WIT import へ渡せない。暗黙の fallback を
避けつつ、実際の codegen/runtime で array→memory→host の境界を検証する必要がある。

## Decision

- `emit_wasm_wasmgc_component_output` を WasmGC backend の明示的な output mode として追加する。
  `print-string` の array reference は一時 local に保持し、`array.len` と `array.get_u` の
  要素ループで memory offset 0 へコピーする。
- コピー前に `ceil(len / 65536)` pages を `memory.grow` し、grow failure は trap とする。memory は
  exported で、bytes は一回の synchronous write 中だけ有効な borrow-like 値とする。
- `create_component_output_import` は
  `lsharp:wasmgc-output/stdout@0.1.0::write` の `(i32, i32) -> ()` を検証し、負値、範囲外、
  pointer+length overflow を拒否してから exported memory を読み取る。host sink の error は
  Wasm trap として返し、WASI import や GC reference import へ fallback しない。
- `run_wasm_wasmgc_component_output_capture` はこの canonical import だけを解決する core runner
  とし、生成した core bytes は `wit/lsharp-wasmgc-output.wit` へ componentize/validate する。

## Evidence

- `wasm_gc_component_output_copies_packed_array_to_linear_memory_import` は `é` の packed bytes
  を GC array から linear memory へコピーし、canonical host sink と exit code を確認する。
- `wasm_gc_component_output_rejects_invalid_linear_memory_range` は `(ptr, len)` の範囲外を拒否する。
- `wasm_gc_component_output_propagates_sink_failure_as_trap` は host sink error の trap 伝播を確認する。
- `wasm_gc_component_output_componentizes_against_wit_world` は実際の core bytes を WIT world へ
  componentize し、Wasmtime Component validation を通す。

## Consequences

- GC reference を canonical boundary へ直接渡さず、guest 側の copy と host 側の range check が
  observable contract になった。
- これは GC→linear-memory と canonical sink の verified partial slice である。WASI
  `fd_write` の partial/error/errno、`flush`/exit ordering、Preview2 actual instantiate/runner、
  Mac/Linux native evidence、selfhost parity は `LEGACY-WASMGC-COMP-IO-01` / `-RUN-01` に残る。
- bytes は opaque のままで、UTF-8/code-point semantics は L# String/runtime 層の別契約とする。
