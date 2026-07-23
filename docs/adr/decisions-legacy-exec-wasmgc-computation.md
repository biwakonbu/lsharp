# ADR: WasmGC computation return and bind boundary

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

Computation Expression は AST と linear lowering に存在するが、WasmGC lowering の `let!` / `do!`
は bind 関数と continuation を呼ばず、式を直列評価してローカルへ格納するだけだった。この挙動は
monadic bind と異なる結果を成功した Wasm として出力し得る。一方、`return` のみの expression は
builder の return 関数を通常の scalar call として実行できる。

## Decision

WasmGC backend では、builder の `return` のみを verified scalar slice として許可する。`let!` / `do!`
を含む computation は、GC closure を continuation として表現する経路が未実装であるため、linear
sequence へ暗黙に落とさず `LS3001` で明示拒否する。linear backend の既存 lowering と macro expansion
の契約は変更しない。

## Evidence

- `test_compile_file_wasmgc_backend_executes_computation_return` が `add-one` を return function として
  `41` に適用し、生成した Wasm を Wasmtime で実行して `42` を確認する。
- `test_wasmgc_backend_rejects_computation_bind_without_gc_closure` が `let!` の silent sequential
  lowering を RED として固定し、`LS3001`、`computation`、`closure` を含む診断を確認する。
- `cargo test -p lsharp-tooling computation` は return の実行と bind 境界の拒否をともに通過する。

## Consequences

- WasmGC の computation return-only scalar slice は実行可能になり、bind 未実装を誤った成功として
  公開しない。
- `let!` / `do!` の実際の bind、funcref/GC closure、multi-step monadic runtime、HKT、WASI/component、
  Mac/Linux native、selfhost compiler は未完了であり、`LEGACY-EXEC-01` と D-04 は active のまま残す。
