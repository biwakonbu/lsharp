# ADR: WasmGC scalar ADT literal pattern

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

ADT constructor と nested pattern は WasmGC struct へ接続済みだったが、payload の literal pattern は
未対応診断へ止まっていた。整数値で constructor を選別する既存の L# semantics を、linear-memory
load に戻さず WasmGC で実行する必要がある。

## Decision

`Int` / `Bool` / `Unit` literal は、constructor payload の `StructGet` 結果と i64 constant を
`I64Eq` で比較する。比較成功時は同じ pattern sequence を続行し、失敗時は次の top-level match arm
へ fallback する。Float/String literal と record pattern はこの slice では `LS3001` で拒否する。

## Evidence

- `test_compile_file_wasmgc_backend_executes_integer_adt_literal_pattern` が `Just 42` / `Just 41` と
  `Set true` の成功・fallback を Wasmtime で実行し、結果 `2` を確認する。
- compile focused 34 件、IR lower 130 件、WasmGC probe 8 件、関連 clippy、docs audit を実行する。

## Consequences

- scalar literal ADT pattern は WasmGC core path で実行できる。
- literal 型全体、GADT、parametric representation、runtime、supported 2 targets、selfhost は
  未完了であり、`LEGACY-EXEC-01` は active のまま残す。
