# ADR: WasmGC scalar String equality

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

Stage 2a で WasmGC の String literal は `StringBytes` (`array<i32>`) へ移ったが、
`string-eq` は linear runtime import `__string_eq` を呼び出すため、WasmGC では
`LS4001` で停止していた。String の比較だけを閉じる、配列再配置を伴わない次の
observable contract が必要だった。

## Decision

WasmGC lowering は二つの `StringBytes` concrete reference を local に保存し、まず
`array.len` を比較する。同じ長さの場合だけ `array.get` を index loop で比較し、全 byte が
一致したときだけ `i64` の Bool `1` を返す。長さ不一致、最初の byte 不一致は `0` とし、
空配列同士は `1` とする。

この slice は既存の `array<i32>` を byte 値の保持に使う。linear backend の tagged pointer
と `__string_eq` runtime helper は変更せず、WasmGC で未対応の concat/substring/print/WASI
を暗黙に linear import へ接続しない。

## Evidence

- `test_compile_file_wasmgc_backend_executes_string_equality` が String parameter を受ける
  `same` 関数を通して、同長一致・同長不一致・長さ不一致・空配列同士を Wasmtime で実行し、結果
  `15` を確認する。
- WasmGC backend focused suite 19 件が成功し、既存 record/ADT/computation/String array slice
  との共存を確認する。
- linear backend の既存 `string-eq` E2E 4 件が成功する。
- `cargo check -p lsharp-ir -p lsharp-wasm -p lsharp-tooling`、対象 crate lib clippy、
  `rustfmt --check`、`git diff --check`、`bash scripts/audit_docs.sh` を gate とする。

## Consequences

- WasmGC の scalar String byte-level equality が actual core Wasm runtime で実行可能になった。
- packed `i8` array、Unicode code-point semantics、concat/substring、print/WASI/component、
  GC array mutation、Mac/Linux native、selfhost compiler は未完了で、`LEGACY-EXEC-01` は
  active のまま残す。
