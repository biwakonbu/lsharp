# ADR: WasmGC typed type-application payload slots

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

WasmGC ADT payload の field resolver は `TypeExpr::Named` だけを受理し、`(Inner Int)` のような
既知型への type application を `LS3001` で拒否していた。また、variant 間で `Int` と `Ref` の
field が同じ ordinal にあると、単一の共通 slot 型へ押し込めず、型不一致を避ける必要があった。

## Decision

type application は head の既知 ADT/record type を runtime representation とし、型引数は実行時に
消去する。ADT の GC struct は variant ごとの field を variant-specific typed slot へ順に配置し、
constructor/pattern の field offset map を共有する。未使用 slot はその field type の zero または
concrete `ref.null` で初期化する。

self-recursive type application は、WasmGC runtime の GC collection 内部 panic を成功経路へ流さないため
明示的に `LS3001` で拒否する。これは GADT refinement、recursive GC runtime、HKT の実装完了とは分離する。

## Evidence

- `test_compile_file_wasmgc_backend_executes_type_application_payload` が `Wrapper (Inner Int)` の
  payload と nested constructor pattern を Wasmtime で実行し、42 を確認する。
- `test_wasmgc_backend_rejects_recursive_type_application_payload_explicitly` が self-recursive
  `(Expr Int)` payload を `LS3001` と `自己参照` 診断で停止する。
- WasmGC backend 17 件、IR lower 130 件、WasmGC probe 8 件、IR/tooling/wasm clippy を実行する。

## Consequences

- 既知型への non-recursive type application payload と variant-specific typed slots が WasmGC core
  path で利用できる。
- GADT の型 refinement、self-recursive representation、Float/String、WASI/component、Mac/Linux
  native、selfhost compiler は未完了であり、`LEGACY-EXEC-01` は active のまま残す。
