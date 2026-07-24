# ADR: WasmGC local non-capturing funcref alias の `call_ref`

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir` WasmGC `let` binding と local function application

## Context

non-capturing lambda literal の direct application は typed `call_ref` へ接続済みだが、`let` に
束縛した値を local funcref として呼ぶ経路は、従来の closure pointer / `CallIndirect` fallback に
落ちる。WasmGC の abstract `funcref` local は、signature-specific `call_ref` が要求する concrete
function reference へ暗黙に昇格できない。

## Decision

- WasmGC の `let` が直接 non-capturing lambda を束縛する場合、`FuncCtx` に concrete function index
  と function type index を記録する。
- `(f arg)` の call site では local `funcref` を `LocalGet` して `CallRef` するのではなく、記録済み
  function index の `RefFunc` を再生成してから、記録済み type index の `CallRef` を生成する。
- `LocalSet` 自体は binding の observable local lifetime を保持するが、call path が abstract local
  の型に依存しないようにする。Linear backend と captured closure の明示拒否は変更しない。
- function parameter、一般 local funcref、captured env struct はこの slice の対象外とし、未対応のまま
  `CallIndirect` fallback へ暗黙に成功させない。

## Evidence

- RED: `wasm_gc_local_non_capturing_lambda_call_lowers_to_call_ref` は、従来 local closure path
  が `LocalGet` / `CallIndirect` を生成するため失敗した。
- GREEN: 同テストは local alias の call site が `RefFunc` / `CallRef` になり、WasmGC lowering が
  linear-memory closure 命令へ戻らないことを確認する。
- Runtime: `wasm_gc_emitter_executes_local_non_capturing_lambda_call_ref` は Wasmtime 29 の
  `wasm_gc(true)`、`wasm_reference_types(true)`、`wasm_function_references(true)` で `42` を実行
  結果として得る。

## Residual risk

local alias は immutable direct lambda の concrete reference に限定される。一般の function
parameter/local funcref、captured env struct、typed function type の deduplication、parametric/nested
closure、trait vtable、WASI/component、Mac Apple Silicon / Linux x86_64 native stage0 と selfhost
parity は未完了であり、`LEGACY-EXEC-01` の aggregate 完了条件には到達していない。
