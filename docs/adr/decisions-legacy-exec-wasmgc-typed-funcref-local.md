# ADR: WasmGC local concrete typed funcref

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir` WasmGC `let` binding と local `call_ref`
- Supersedes: `decisions-legacy-exec-wasmgc-local-funcref-call.md` の暫定 call-site 再 materialize

## Context

Stage 3f では、`let` に束縛した non-capturing lambda を呼び出すために、`FuncCtx` が function index と
function type index を別管理し、call site で `ref.func` を再生成してから `call_ref` していた。
これは WasmGC の abstract `funcref` local が signature-specific な concrete reference として
`call_ref` に渡せないための安全な暫定策だったが、binding した値を local に保存しているのに call
site で元の関数を再 materialize するため、IR の値表現と call の observable contract が一致しない。

## Decision

- `IrType::TypedFuncRef(u32)` を追加し、IR の型セクション上の concrete function type index を保持する。
- WasmGC の `let` が直接 non-capturing lambda を束縛する場合、local の型を `TypedFuncRef` として
  `RefFunc → LocalSet` に保存する。function index を別の `FuncCtx` map に複製しない。
- `(f arg)` の call site は引数を評価した後に `LocalGet → CallRef(type_index)` を生成する。
  `RefFunc` の call-site 再生成や linear-memory `CallIndirect` fallback は行わない。
- WasmGC emitter は `TypedFuncRef` を concrete `ref null $function_type` として出力する。
  `print-string` の synthetic import により function type section がずれる場合は、他の
  `RefFunc` / `CallRef` と同じ import offset を適用する。
- abstract `FuncRef`、一般の function parameter/local、captured env struct、module-link 後の
  typed local remap はこの slice の対象外とし、未対応のまま成功させない。

## Evidence

- RED: `wasm_gc_local_non_capturing_lambda_call_lowers_to_call_ref` は、暫定実装の
  `RefFunc → CallRef` call site を期待値と区別し、`LocalGet → CallRef` がないため失敗した。
- GREEN: 同テストは `RefFunc → LocalSet → I64Const(41) → LocalGet → CallRef` を確認し、
  call site の `RefFunc` 再 materialize がないことを固定する。
- Runtime: `wasm_gc_emitter_executes_local_non_capturing_lambda_call_ref` は Wasmtime 29 の
  `wasm_gc(true)`、`wasm_reference_types(true)`、`wasm_function_references(true)` で `42` を実行する。
- Import offset: `wasm_gc_emitter_offsets_local_typed_funcref_after_print_string_import` は
  synthetic `print-string` import を含む module の validation を確認する。

## Residual risk

この slice は immutable な direct non-capturing lambda alias に限られる。一般の typed funcref 値、
function parameter/local、captured env struct、parametric/nested closure、trait vtable、WASI/component、
Mac Apple Silicon / Linux x86_64 native stage0 と selfhost parity は未完了であり、`LEGACY-EXEC-01`
の aggregate 完了条件には到達していない。
