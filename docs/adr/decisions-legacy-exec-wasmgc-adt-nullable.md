# ADR: WasmGC nullable ADT reference payload

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

nested ADT payload は concrete `IrType::Ref` として実行できるようになったが、`Present Maybe` と
`Empty` のように variant ごとに payload 数が異なる ADT では、共通 struct slot に値がない variant
の初期値を用意する必要があった。欠損 Ref を i64 zero にすると WasmGC の型契約を壊す。

## Decision

IR に `RefNull(type_idx)` を追加し、GC type index remap、WasmGC validator/emitter、linear backend の
互換 fallback を固定する。ADT constructor は共通 slot の型が `Ref(type_idx)` で、対象 variant に
field がない場合、その slot に `ref.null concrete(type_idx)` を積んで `StructNew` を実行する。

tag/pattern lowering は既存の nested path を使い、null payload を含む nested constructor の不一致は
次 arm へ進む。String、GADT、parametric 表現、root/allocator/runtime はこの boundary の外側に残す。

## Evidence

- `test_compile_file_wasmgc_backend_executes_nullable_adt_payload` が
  `Present (Just 42)` と `Present Nothing` を構築し、nested `Just` の不一致から wildcard arm へ
  fallback する結果 `42` を Wasmtime で確認する。
- compile focused 33 件、IR lower 130 件、WasmGC probe 8 件、`lsharp-ir` / `lsharp-tooling` /
  `lsharp-wasm` の clippy、docs audit を実行する。

## Consequences

- 欠損 concrete Ref payload を型安全な nullable reference として生成できる。
- `RefNull` は backend-specific な WasmGC capability であり、linear path は互換的な i64 zero
  fallback に留まる。`LEGACY-EXEC-01` は未完了で、supported 2 targets と selfhost evidence は
  残る。
