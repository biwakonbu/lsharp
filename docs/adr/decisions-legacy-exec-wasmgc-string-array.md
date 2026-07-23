# ADR: WasmGC scalar String GC array

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

WasmGC の record/ADT は concrete reference を使えるようになったが、String literal はまだ
`__alloc`、linear memory、runtime import を経由していた。そのまま WasmGC へ渡すと `LS4001` で
止まるか、別の i64 表現へ暗黙に戻るため、文字列の値表現を GC array へ移す最小 contract が必要だった。

## Decision

WasmGC lowering は program の既存 record/ADT type index を変更せず、末尾に `StringBytes` の
`array<i32>` type を予約する。UTF-8 bytes は `array.new_fixed` の scalar elements として生成し、
`string-length` は `array.len`、`string-char-at` は `array.get` へ lowering する。String の関数
parameter/record field も同じ concrete array reference type を使う。linear backend の tagged pointer
representation は維持する。

この slice は `array<i32>` を選ぶことで `wasm-encoder 0.245` の value array API に限定し、packed
`i8` storage、data segment、WASI/print bridge を後続タスクへ残す。未対応の concat/substring/file I/O
を WasmGC で linear runtime import に暗黙接続しない。

## Evidence

- `test_compile_file_wasmgc_backend_executes_string_array_length` が `"hello"` の `array.len` を
  Wasmtime で実行して `5` を確認する。
- `test_compile_file_wasmgc_backend_executes_string_array_get` が `array.get` で `"hello"[1]` を
  読み、`101` (`e`) を確認する。
- `test_compile_file_wasmgc_backend_passes_string_array_to_user_function` が String parameter の
  concrete GC reference signature と user call を実行して `5` を確認する。
- WasmGC backend focused suite 18 件、`cargo check -p lsharp-ir -p lsharp-wasm -p lsharp-tooling`、
  `git diff --check` を実行する。

## Consequences

- String literal/length/byte access と String parameter の WasmGC core slice が実行可能になった。
- packed `i8` array、Unicode code-point semantics、concat/substring/equality、print/WASI/component、
  GC array mutation、Mac/Linux native、selfhost compiler は未完了で、D-01 と `LEGACY-EXEC-01` は
  active のまま残す。
