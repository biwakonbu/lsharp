# ADR: WasmGC closure lowering の明示的な未実装境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir` WasmGC `Expr::Lambda` lowering

## Context

現行の lambda lifting は、closure を linear memory の tagged object として確保し、
`FuncIdx`、`I32Store`、captured value の `I64Store`、呼び出し側の `CallIndirect` を生成する。
WasmGC emitter はこれらの linear-memory 命令を受け付けないため、WasmGC backend に同じ
lowering を流すと、後段まで不正な IR が進んで failure boundary が遅れる。

## Decision

- `LowerBackend::WasmGc` の `Expr::Lambda` は、現段階では
  `LowerError::Unsupported` (`typed funcref/env struct` を含む診断) を返す。
- Linear backend の既存 lambda lifting と closure runtime は変更しない。
- typed `RefFunc` / `CallRef` emitter capability と module-link remap は別 ADR の verified
  partial slice として再利用し、closure env struct 実装時にこの境界を置き換える。

## Evidence

- RED: `wasm_gc_closure_lowering_rejects_linear_memory_fallback_explicitly` の fixture は、従来
  `Call(1)`、`I32Store`、`FuncIdx` を含む linear-memory closure object を生成した。
- GREEN: 同テストは lowering 段階で `LowerError::Unsupported` を受け取り、WasmGC emitter へ
  不正な fallback IR を渡さない。

## Residual risk

これは安全な拒否境界であり、closure 対応の完了ではない。env struct field layout、captured
value の GC root/field semantics、typed `CallRef` type allocation、closure call lowering、
`examples/hkt.ls` / `examples/computation.ls` の WasmGC runtime、Mac/Linux native/selfhost parity
は未完了である。
