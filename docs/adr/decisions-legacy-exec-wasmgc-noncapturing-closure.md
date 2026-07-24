# ADR: WasmGC non-capturing lambda の typed funcref lowering

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir` WasmGC `Expr::Lambda` lowering

## Context

WasmGC の closure lowering を一度に完成させる前に、自由変数を持たない lambda は環境を必要と
しない。従来の lambda lifting は自由変数の有無にかかわらず linear-memory の closure object を
確保していたため、WasmGC backend でその経路を通すと `FuncIdx`、store 命令、allocator が後段へ
流れ、不正な Wasm または誤った i64 fallback になる。

## Decision

- `LowerBackend::WasmGc` の non-capturing `Expr::Lambda` は、元の引数だけを持つ lifted function
  と `Instruction::RefFunc` へ lowering する。
- WasmGC の `Type::Fun` は `IrType::FuncRef` とし、親関数の lambda 値を typed `funcref` として
  保持する。runtime import の論理 function index は core module の user function index へ
  lowering 時に明示変換する。
- captured lambda は Stage 3c の `LowerError::Unsupported` 境界を維持する。env struct、captured
  field、closure call (`CallRef`) は別の observable contract として後続 task に分離する。
- Linear backend の既存 lambda lifting / tagged closure object は変更しない。

## Evidence

- RED: `wasm_gc_non_capturing_lambda_lowers_to_funcref` は、既存の「全 lambda 拒否」境界で失敗した。
- GREEN: 同テストは親関数の `IrType::FuncRef`、`RefFunc`、元引数だけの lifted function、
  linear-memory closure 命令の不在を確認する。
- captured boundary: `wasm_gc_closure_lowering_rejects_linear_memory_fallback_explicitly` は
  自由変数 `n` を持つ lambda を引き続き明示拒否する。
- WasmGC gate: `wasm_gc_emitter_accepts_lowered_non_capturing_lambda_funcref` は生成 IR を
  `wasm_gc(true)`、`wasm_reference_types(true)`、`wasm_function_references(true)` の Wasmtime
  29 engine で検証する。typed `call_ref` の実行 capability は
  `wasm_gc_emitter_executes_typed_funcref_call_ref` で別途確認済みである。

## Residual risk

non-capturing lambda の値は生成・検証できるが、L# source から funcref を `CallRef` で呼び出す
closure call は未実装である。captured env struct、typed function type allocation、nested/parametric
closure、trait vtable、WASI/component、Mac Apple Silicon / Linux x86_64 native stage0 と selfhost
parity は未完了であり、`LEGACY-EXEC-01` の aggregate 完了条件には到達していない。
