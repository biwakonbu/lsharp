# ADR: WasmGC typed funcref emitter capability

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-wasm` WasmGC emitter の `RefFunc` / `CallRef`

## Context

Stage 3 の closure lowering は、現在も env を linear-memory tuple として扱う
`CallIndirect` 経路に依存している。一方、WasmGC の typed function references を emitter から
利用できるかは独立した runtime capability であり、closure lowering と同じ変更へ混ぜると
failure boundary が分からなくなる。

## Decision

- WasmGC emitter は IR の `RefFunc(function_index)` を `ref.func` として出力する。
- IR 内で参照される function index を検証し、参照関数を declared element segment に登録する。
- IR の `CallRef(type_index)` を `call_ref` として出力し、関数型インデックスの範囲を検証する。
- Wasmtime runtime probe では `wasm_gc(true)` に加え、依存する `wasm_reference_types(true)` と
  `wasm_function_references(true)` を明示する。proposal flag を暗黙の default にしない。
- この ADR は hand-written IR の emitter/runtime capability だけを受け入れ、closure env
  lowering、module link remap、trait vtable、native/selfhost parity は受け入れない。

## Evidence

- RED: `wasm_gc_emitter_executes_typed_funcref_call_ref` は `RefFunc` が未対応のため失敗した。
- GREEN: 同テストは WasmGC module の生成、Wasmtime 29 の validate/instantiate、`call_ref` 実行を
  通過し、`i64` の `41` を返す。
- Regression: `cargo test -p lsharp-wasm --test wasmgc_probe -- --test-threads=1` は 92 tests
  passed。

## Residual risk

`lower/closure.rs` はまだ `CallIndirect` を生成し、closure env は linear-memory 表現のままで
ある。module-link 時の funcref index/type remap、synthetic import を含む index 空間の統合、
`examples/hkt.ls` / `examples/computation.ls` の closure E2E、Mac Apple Silicon / Linux x86_64
native stage0 と selfhost parity は未検証である。したがって Stage 3 と
`LEGACY-EXEC-01` の完了証拠には拡大解釈しない。
