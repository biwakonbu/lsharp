# ADR: WasmGC captured env struct の direct `call_ref`

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC の captured lambda direct application
- Related: `decisions-legacy-exec-wasmgc-closure-boundary.md`, `decisions-legacy-exec-wasmgc-typed-funcref-local.md`

## Context

現行の lambda lifting は captured value を linear memory の closure object に保存し、`FuncIdx`、
`I64Load`/`I64Store`、`CallIndirect` で呼び出す。WasmGC backend がこの IR をそのまま受け入れると、
typed funcref/env struct へ移行していない closure が誤って Wasm を生成するため、captured lambda は
Stage 3c で明示拒否していた。

## Decision

- captured lambda を direct application する場合だけ、GC struct env を生成する。
- env struct の field 0 は lifted function の concrete `TypedFuncRef`、field 1 以降は自由変数を
  安定ソートした順の captured local とする。
- lifted function の末尾に `Ref(env_type)` parameter を追加し、function body の先頭で
  `StructGet` した capture を local に復元する。
- call site は `args → env_ref → StructGet(field 0) → CallRef` とし、linear-memory closure
  pointer、`FuncIdx`、`CallIndirect` へ fallback しない。
- env struct が後続の function type を concrete heap type として参照するため、WasmGC emitter は
  typed funcref を含む type section を GC type と function type の recursive type group として
  materialize する。IR index は従来の GC → import → user function 順序を維持し、synthetic
  `print-string` import offset は `TypedFuncRef`、`RefFunc`、`CallRef` に一貫して適用する。
- captured lambda の `let` alias、関数の戻り値、一般 function parameter/local、nested/parametric
  closure はこの slice の対象外とし、Stage 3c の明示拒否を維持する。

## Evidence

- RED: `wasm_gc_captured_lambda_direct_call_lowers_to_env_struct_call_ref` は、従来の
  `LowerError::Unsupported` 境界で失敗した。
- GREEN: 同テストは `StructNew` / `CallRef` を生成し、`CallIndirect`、`FuncIdx`、
  `I64Load`、`I64Store` を含まないことを確認する。
- Runtime: `wasm_gc_emitter_executes_captured_lambda_env_struct_call_ref` は Wasmtime 29 の
  `wasm_gc(true)`、`wasm_reference_types(true)`、`wasm_function_references(true)` で `n=1` の
  `42` を実行する。
- Import offset: `wasm_gc_emitter_offsets_captured_env_funcref_after_print_string_import` は
  synthetic `print-string` import 付き module の validation を確認する。

## Residual risk

この決定は direct captured call の verified partial slice に限られる。closure を値として返す/束縛する
経路、一般の higher-order function、GC reference rooting と nested env、trait vtable、WASI/component、
Mac Apple Silicon / Linux x86_64 native stage0 と selfhost parity は未完了であり、`LEGACY-EXEC-01`
の aggregate 完了条件には到達していない。
