# ADR: WasmGC `print-string` external import boundary

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

WasmGC の String は nullable な `PackedByteArray` reference だが、lowering が予約する
`Call(4)` は従来 WasmGC emitter で未対応 runtime import として拒否されていた。一方で、
`print-string` を i64 pointer の linear-memory ABI として暗黙に通すと、GC reference と host
runtime の境界が曖昧になり、無効な module や誤った stdout 成功を作る。

## Decision

WasmGC backend では次の boundary だけを materialize する。

- `Call(4)` を `print-string` の logical runtime index として扱う。
- `Module.gc_types` の `PackedByteArray` を StringBytes の concrete heap type とし、function type
  `(ref null $StringBytes) -> ()` を追加する。
- synthetic import `env.print-string` は `Call(4)` が存在する module にだけ追加する。既存の
  `Module.imports` は先に保持し、synthetic import の後ろへ user function index を remap する。
- `Call(4)` 以外の未対応 runtime logical index、`CallImport`、WASI/component/native の host
  integration は explicit codegen error とする。linear backend の import ABI は変更しない。

この ADR の `print-string` は import の型・validation・instantiate boundary までを意味する。
host callback が GC array を読み、stdout または WASI fd_write へ出力する意味論は後続 ADR の対象
とし、stub import の instantiate 成功を print 実装完了とは扱わない。

## Evidence

- `wasm_gc_emitter_materializes_print_string_import_boundary` が生成 module の Wasmtime
  validation、`env.print-string` の concrete reference parameter、stub import による
  instantiate、`main` actual execution を確認する。
- `test_compile_file_wasmgc_backend_emits_print_string_import` が source lowering から
  `--backend=wasmgc --target=web-wasm` module を生成し、同じ import 名を確認する。
- WasmGC probe 11 件、WasmGC tooling backend 29 件が成功した。
- `cargo check -p lsharp-ir -p lsharp-wasm -p lsharp-tooling`、対象 crate の
  `clippy -D warnings`、`git diff --check`、`scripts/audit_docs.sh` を成功させる。

## Consequences

- GC String reference を i64 pointer へ暗黙変換せず、host と compiler の最初の ABI を検証可能な
  import として固定できる。
- `print-string` の実出力、GC array byte read の host API、WASI/component adapter、native/selfhost
  runtime、Unicode code-point semantics、supported target の actual evidence は未完了である。
  `LEGACY-EXEC-01` は active のまま残す。
