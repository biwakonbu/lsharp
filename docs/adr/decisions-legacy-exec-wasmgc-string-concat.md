# ADR: WasmGC scalar String concatenation

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

Stage 2a/2b で String literal、length、byte access、equality は `StringBytes`
(`array<i32>`) を使えるようになったが、`string-concat` は root 操作と linear runtime import
`__string_concat` に依存していた。動的な長さの結果を GC array だけで生成する contract が必要だった。

## Decision

WasmGC lowering は lhs/rhs を concrete array reference local に保持し、二つの `array.len` を
加算した長さで `array.new_default` を実行する。lhs は index 0 から、rhs は lhs の長さを offset
にして、`array.get` した `i32` byte を `array.set` で結果へコピーする。空文字列も同じ経路で扱う。

`ArrayNewDefault` と `ArraySet` は WasmGC 専用 IR 命令として追加し、linear emitter では明示的な
codegen error にする。linear backend の root/import/runtime 表現は変更しない。

## Evidence

- `test_compile_file_wasmgc_backend_executes_string_concat` が String parameter を受ける `join`
  関数を通し、`"hello" + " world"` の length と `"a" + "b"` の byte access を Wasmtime で実行し、
  `109` を確認する。
- WasmGC backend focused suite 20 件が成功する。
- 既存 linear concat E2E 10 件が成功する。
- `cargo check -p lsharp-ir -p lsharp-wasm -p lsharp-tooling`、対象 crate lib clippy、
  `rustfmt --check`、`git diff --check`、`bash scripts/audit_docs.sh` を gate とする。

## Consequences

- WasmGC の scalar String byte-level concatenation が actual core Wasm runtime で実行可能になった。
- packed `i8` array、Unicode code-point semantics、substring、print/WASI/component、GC array
  mutation、Mac/Linux native、selfhost compiler は未完了で、`LEGACY-EXEC-01` は active のまま残す。
