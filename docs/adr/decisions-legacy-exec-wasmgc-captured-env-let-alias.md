# ADR: WasmGC captured env の `let` alias `call_ref`

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: WasmGC の captured lambda `let` binding と同一関数内の呼び出し
- Related: `decisions-legacy-exec-wasmgc-captured-env-direct-call.md`, `decisions-legacy-exec-wasmgc-typed-funcref-local.md`

## Context

Stage 3h は captured lambda を source-level で直接 application する場合だけ GC env struct と
typed `call_ref` を生成した。一方、`let` に束縛した同じ closure は従来の linear-memory closure
fallback へ進むか、WasmGC の明示拒否に戻るため、source-level の alias 呼び出しが direct call と
同じ observable contract を持っていなかった。

## Decision

- WasmGC の `let` binding が captured `Expr::Lambda` を直接値として持つ場合、Stage 3h と同じ
  env struct (`field 0 = TypedFuncRef`, 後続 field = captured local) を生成する。
- binding の local type は `Ref(env_type)` とし、abstract `funcref` に潰さない。これにより
  env の concrete function reference と capture payload の型を local 境界で保持する。
- alias の application は `args → LocalGet(env) → StructGet(field 0) → CallRef` とし、
  `CallIndirect`、`FuncIdx`、linear-memory closure pointer を生成しない。
- env type が後続 function type を参照する recursive type group、`print-string` synthetic import の
  type/function index offset は Stage 3h の emitter 契約を再利用する。
- closure を関数の戻り値にする経路、function parameter/local への一般 higher-order 受け渡し、
  nested/parametric closure はこの slice では有効化せず、明示拒否を維持する。

## Evidence

- RED: `wasm_gc_captured_lambda_let_alias_lowers_to_env_struct_call_ref` は、実装前に
  `LowerError::Unsupported` の captured closure 境界で失敗した。
- GREEN: 同テストは `StructNew`、`LocalSet`、`LocalGet`、`StructGet`、`CallRef` を確認し、
  `CallIndirect`、`FuncIdx`、`I64Load`、`I64Store` を含まない。
- Runtime: `wasm_gc_emitter_executes_captured_lambda_let_alias_call_ref` は Wasmtime 29 の
  WasmGC + reference types + typed function references で `n=1` の `42` を実行する。
- Regression: WasmGC probe 100 件、IR focused tests、clippy、docs audit を通過し、direct captured
  call と typed local の既存契約も維持した。

## Residual risk

この決定は同一関数内の直接 `let` alias に限定された verified partial slice である。closure の
戻り値、一般 higher-order function、function parameter/local、nested env と GC rooting、module-link
後の env remap、trait vtable、WASI/component、Mac Apple Silicon / Linux x86_64 native stage0 と
selfhost parity は未完了で、`LEGACY-EXEC-01` の aggregate 完了条件には到達していない。
