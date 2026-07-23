# ADR: WasmGC record pattern field checks

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

WasmGC の record literal/access は GC struct へ接続済みだったが、record pattern は変数束縛だけを
行い、field literal を含む arm を `LS3001` で拒否していた。record pattern の不一致を linear-memory
表現へ戻さず、ADT pattern と同じく次の arm へ進める必要がある。

## Decision

record pattern の各 field を concrete GC struct の typed local として sequence 化する。
`Int` / `Bool` / `Unit` は `I64Eq` で比較し、成功時は残りの field checks と body を続け、失敗時は
同じ scrutinee の次の top-level match arm へ fallback する。nested record pattern は child struct の
field sequence を同じ continuation に連結する。Float/String の value representation はこの slice
の対象外とし、暗黙の i64 比較へ落とさず `LS3001` で拒否する。

## Evidence

- `test_compile_file_wasmgc_backend_executes_record_literal_pattern_with_fallback` が direct record
  の `42` 一致と `41` 不一致を Wasmtime で実行する。
- `test_compile_file_wasmgc_backend_executes_nested_record_literal_pattern` が nested record の
  `42` 一致と `41` fallback を Wasmtime で実行する。
- `test_wasmgc_backend_rejects_unsupported_record_string_literal_pattern` が未対応 String literal
  を `LS3001` で停止する。
- WasmGC compile backend 15 件、IR lower 130 件、WasmGC probe 8 件を実行し、既存 linear path
  は `LowerBackend::Linear` の focused tests で維持した。

## Consequences

- direct/nested record pattern の scalar literal と field binding を WasmGC core path で実行できる。
- nominal runtime cast、Float/String representation、WASI/component、Mac/Linux native、selfhost
  compiler は未完了であり、`LEGACY-LANG-01` と `LEGACY-EXEC-01` は active のまま残す。
