# ADR: WasmGC output Component の actual instantiate

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp:wasmgc-output` custom world の Component host/runner

## Context

Stage 2m で core module の canonical `(ptr, len)` output、Stage 2n で writer/fd_write semantics を
固定した。Component artifact を validation するだけでは公開 runner の証拠にならないため、WIT
world を実際に instantiate し、`list<u8>` lift と `main: s64` export を host から呼び出す必要がある。

## Decision

- `run_wasm_wasmgc_component_output_component_with_stdout_sink` は Wasmtime Component API の
  `Linker` に `lsharp:wasmgc-output/stdout@0.1.0` interface と `write(list<u8>)` を定義する。
- Component boundary で lift された `Vec<u8>` を sink へ一回渡し、sink error は Component trap と
  して伝播する。WASI Preview1/Preview2 linker への暗黙 fallback は行わない。
- `main` export は Component `Val::S64` として呼び出し、i32 exit code へ checked conversion する。
  `run_wasm_wasmgc_component_output_component_capture` は stdout と exit code を capture する。

## Evidence

- `wasm_gc_component_output_component_runner_executes_wit_host` は生成 core bytes の componentize、
  validation、custom host link、`list<u8>` の output、s64 exit code を actual runtime で確認する。
- `wasm_gc_component_output_component_runner_propagates_sink_failure` は Component host sink error
  が trap として返ることを確認する。

## Consequences

- custom WIT world の Component runner が synthetic import/validation だけでなく actual instantiate
  と export call まで到達した。
- これは `wasmgc-output` world の verified partial slice であり、WASI Preview2 `wasi:cli/run`、
  fd table/rights、Mac Apple Silicon/Linux x86_64 artifact/runtime、native/selfhost parity は
  `LEGACY-WASMGC-COMP-RUN-01` に残る。
