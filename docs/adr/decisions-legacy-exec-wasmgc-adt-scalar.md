# ADR: WasmGC scalar ADT constructor/pattern の最小 lowering

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

WasmGC backend は record の direct construction/access まで接続済みだったが、ADT constructor と
constructor pattern は linear-memory の pointer/tag 操作を生成していた。そのまま WasmGC emitter
へ渡すと、未対応の `i32.load` を含む module になり、WasmGC の実行境界を検証できない。

## Decision

非パラメトリックで payload が scalar `Int` の ADT について、TypeDef ごとに WasmGC struct を登録する。
struct の field 0 を `i64` tag、field 1 以降を variant 間で共有する `i64` payload slot とし、constructor
は `StructNew`、pattern は tag の `StructGet` + `I64Eq` と payload `StructGet` を使う。`Var` と wildcard
だけを受け付け、nested/literal pattern、型付き payload、GADT、parametric ADT は明示的な未対応診断で
停止する。

WasmGC のこの slice では、user-call 引数に linear GC root 操作を挿入しない。root stack、allocator、
strings、collections、WASI/component は別の境界として維持する。

## Evidence

- `crates/lsharp-tooling/src/compile.rs::test_compile_file_wasmgc_backend_executes_adt_constructor_and_match`
  が `(Just 42)` と `Nothing` の両方を WasmGC struct として構築し、pattern match の結果を Wasmtime
  で実行して `42` を確認する。
- WasmGC probe、IR lowering の focused tests、`lsharp-ir` / `lsharp-tooling` の clippy を同じ commit
  で検証する。

## Consequences

- scalar ADT の constructor/pattern は record と同じ core WasmGC path で実行できる。
- payload を一律 `i64` slot とするため、nested/type-specific payload と parametric representation は
  まだ表現できない。
- `LEGACY-EXEC-01` は未完了であり、TODO の ADT/runtime/root/target 要件は削除しない。
