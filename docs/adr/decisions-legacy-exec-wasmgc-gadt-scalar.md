# ADR: WasmGC scalar GADT refinement execution

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

GADT variant の return type は parser/type inference に既に保持されていたが、WasmGC の実行証跡は
存在せず、`examples/gadt.ls` は型チェックだけに留まっていた。variant-specific typed slot により
`Int` と `Bool` payload を同じ ADT の異なる field として安全に出力できるようになったため、
non-recursive scalar GADT の observable contract を固定する。

## Decision

GADT の return type refinement は型推論の責務として維持し、runtime representation は通常 ADT と
同じ tagged GC struct を使う。variant ごとの typed slot と tag pattern を組み合わせ、`Expr Int` と
`Expr Bool` の各 constructor/pattern を WasmGC core path で実行する。異なる refinement の constructor
を関数へ渡す不一致は `LS1004` の compile-time diagnostic とする。

self-recursive GADT payload と recursive evaluator は Wasmtime 29 の GC runtime boundary が未解決のため
この ADR の完了条件に含めず、前段の type-application ADR の `LS3001` boundary を維持する。

## Evidence

- `test_compile_file_wasmgc_backend_executes_scalar_gadt_refinement` が `IntLit 42` と `BoolLit true`
  を別々の refined function で実行し、結果 43 を Wasmtime で確認する。
- `get-int (BoolLit true)` の RED は `[LS1004]` となり、refinement を i64 表現へ暗黙に消去しない。
- WasmGC compile focused suite、IR lower 130 件、GADT type inference tests、WasmGC probe、clippy、
  docs audit を実行する。

## Consequences

- non-recursive scalar GADT の constructor/pattern/runtime slice と refinement diagnostic が固定される。
- recursive GADT representation/evaluator、HKT、Float/String、WASI/component、Mac/Linux native、
  selfhost compiler は未完了であり、`LEGACY-EXEC-01` は active のまま残す。
