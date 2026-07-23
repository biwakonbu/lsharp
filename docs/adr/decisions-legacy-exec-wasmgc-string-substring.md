# ADR: WasmGC scalar String substring

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

Stage 2a〜2c で String literal、length、byte access、equality、concat は
`StringBytes` (`array<i32>`) を使えるようになったが、`substring` は source を root して
linear memory の `memory.copy` と `__alloc` を使うため、WasmGC では runtime import 境界に到達していた。

## Decision

WasmGC lowering は valid byte range の source/start/end を local に保存し、`end - start` の
長さで `array.new_default` を実行する。index loop の各反復で source の `start + index` を
`array.get` し、結果へ `array.set` する。`start == end` は空 array を返す。範囲外や負値の
診断はこの slice の契約外として後続タスクに残す。

linear backend の tagged pointer、root 操作、`memory.copy`、`__alloc` は変更しない。

## Evidence

- `test_compile_file_wasmgc_backend_executes_string_substring` が String parameter を受ける
  `slice` 関数を通し、`"hello world"[6..11]` の length/byte access と empty range を Wasmtime
  で実行し、結果 `116` を確認する。
- WasmGC backend focused suite 21 件が成功する。
- 既存 linear substring E2E の実行結果を確認する（既存 Linux x86 metadata test の無関係な
  fixture marker failure は別 issue として残る）。
- `cargo check -p lsharp-ir -p lsharp-wasm -p lsharp-tooling`、対象 crate lib clippy、
  `rustfmt --check`、`git diff --check`、`bash scripts/audit_docs.sh` を gate とする。

## Consequences

- WasmGC の valid scalar String byte-range substring が actual core Wasm runtime で実行可能になった。
- invalid range diagnostics、packed `i8` array、Unicode code-point semantics、print/WASI/component、
  GC array mutation、Mac/Linux native、selfhost compiler は未完了で、`LEGACY-EXEC-01` は active
  のまま残す。
