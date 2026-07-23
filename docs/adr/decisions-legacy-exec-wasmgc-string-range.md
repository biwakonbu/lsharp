# ADR: WasmGC substring range boundary

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

Stage 2d の WasmGC `substring` は valid byte range だけを想定し、`end - start` を先に
計算していた。そのため負値、逆順、source length 超過、i64 の巨大値が i32 へ wrap してから
allocation/array access へ進む危険があり、invalid input の境界が明示されていなかった。

## Decision

WasmGC lowering は `substring` の start/end を i64 local に保持したまま、次の条件を検証する。

- `start >= 0`
- `end >= 0`
- `start <= end`
- `end <= source byte length`

source/start/end がすべて compile-time literal の場合、条件違反は `LowerError::Unsupported`
(`LS3001`) と source span 付き診断で停止する。動的な値の場合は同じ検証を WasmGC IR の
`if(unreachable)` guard として出力し、違反時に `wasmtime::Trap::UnreachableCodeReached` で
停止する。検証後にだけ i32 へ変換し、既存の packed `StringBytes` array copy を続ける。

linear backend の tagged pointer、`memory.copy`、runtime import と既存 substring semantics は
変更しない。

## Evidence

- `test_wasmgc_substring_rejects_static_invalid_range` が逆順の literal range を `LS3001`、source
  span 付きで拒否する。
- `test_wasmgc_substring_emits_dynamic_range_trap` が dynamic bounds の IR に `Unreachable` を
 含むことを固定する。
- `test_compile_file_wasmgc_backend_traps_dynamic_invalid_substring_ranges` が負値、逆順、終端超過
  の 3 fixture を actual Wasmtime で実行し、全て `UnreachableCodeReached` になることを確認する。
- WasmGC backend focused suite 28 件、substring lowering 4 件、`cargo check -p lsharp-ir -p
  lsharp-wasm -p lsharp-tooling` を成功させる。
- 既存 linear substring E2E は 9 件が成功した。併走する Linux x86 metadata fixture は従来どおり
  `marker=0` の無関係な失敗を残しており、この slice の変更起因とは扱わない。

## Consequences

- WasmGC substring は invalid byte range を allocation/array access へ進めず、compile-time または
  explicit runtime trap で fail-closed になった。
- 動的 invalid range の trap 本文を利用者向け structured diagnostic に変換すること、Unicode
  code-point semantics、print/WASI/component bridge、GC mutation の公開契約、supported target の
  native evidence、selfhost compiler は未完了である。`LEGACY-EXEC-01` は active のまま残す。
