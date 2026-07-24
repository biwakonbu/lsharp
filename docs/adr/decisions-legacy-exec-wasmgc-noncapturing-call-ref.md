# ADR: WasmGC non-capturing lambda の source-level `call_ref`

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir` WasmGC direct lambda application と `lsharp-wasm` index materialization

## Context

non-capturing lambda の値は typed `funcref` として生成できても、source-level application が
従来の generic indirect-call path へ落ちると、linear-memory closure pointer と `CallIndirect` が
再び生成される。WasmGC backend は env struct や linear memory closure をまだ持たないため、環境を
必要としない direct application だけを typed `CallRef` へ接続する。

## Decision

- WasmGC の `Expr::App` が直接 non-capturing `Expr::Lambda` を受け取った場合、引数を先に評価し、
  lambda の `RefFunc`、その function type index の `CallRef` を生成する。
- function type index は WasmGC type section の `GC type → import function type → user function
  type` 順序に合わせ、lowerer が保持する user function index に GC type 数を加えて求める。
- emitter が `print-string` の synthetic import を materialize する場合は、`RefFunc` の element
  index と `CallRef` の function type index の双方に同じ synthetic import offset を加える。
- captured closure、local funcref の generic indirect call、env struct、linear backend の既存
  lambda lifting はこの slice へ混ぜない。captured lambda は Stage 3c の明示拒否を維持する。

## Evidence

- RED: `wasm_gc_non_capturing_lambda_call_lowers_to_call_ref` は、従来の間接的な関数呼び出し拒否で
  失敗した。
- GREEN: 同テストは `I64Const(41) → RefFunc(1) → CallRef(2)` を確認する。`2` は現在の
  `StringBytes` GC type を含む type section の function type index である。
- Runtime: `wasm_gc_emitter_executes_lowered_non_capturing_lambda_call_ref` は Wasmtime 29 の
  `wasm_gc(true)`、`wasm_reference_types(true)`、`wasm_function_references(true)` で `42` を実行
  結果として得る。
- Synthetic import: `wasm_gc_emitter_offsets_lowered_lambda_call_ref_after_print_string_import`
  は `print-string` import が挿入される module を同じ engine で検証する。

## Residual risk

lambda literal 以外の local funcref application、captured env struct、typed function type の
deduplication、nested/parametric closure、trait vtable、WASI/component、Mac Apple Silicon /
Linux x86_64 native stage0 と selfhost parity は未完了であり、`LEGACY-EXEC-01` の aggregate 完了
条件には到達していない。
