# ADR: WasmGC nested ADT payload と pattern の最小 lowering

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `--backend=wasmgc --target=web-wasm` の Rust compiler path

## Context

scalar ADT は WasmGC struct の共通 `i64` payload slot で実行できるようになったが、ADT を別の
ADT に保持する payload は `i64` に暗黙変換できず、nested constructor pattern も linear-memory
fallback に落ちていた。未対応表現を成功に見せず、型付き Ref の境界を固定する必要がある。

## Decision

program 内の ADT struct index を先に予約し、`TypeExpr::Named` の ADT/record payload は
`IrType::Ref` として共通 struct slot に保持する。variant field type は slot 間で一致する必要が
あり、欠損 variant を含む nullable reference slot、String、parametric/GADT 表現は `LS3001` で
拒否する。

constructor pattern は親 payload を typed local に取り出し、子 constructor の tag/payload を
再帰的に検査する。nested pattern が失敗した場合は同じ scrutinee の次の arm へ進み、pattern variable
には source type name を引き継いで、その値を次の `match` の scrutinee として使えるようにする。

## Evidence

- `test_compile_file_wasmgc_backend_executes_nested_adt_constructor_and_pattern` は
  `Box (Just 42)` の nested match 成功と `Box Nothing` の wildcard fallback を Wasmtime で実行し、
  結果 `42` を確認する。
- `test_compile_file_wasmgc_backend_preserves_nested_adt_binding_type` は `(Box inner)` の Ref
  binding を次の `match` へ渡す経路を同じ WasmGC/Wasmtime E2E で確認する。
- unresolved String payload と literal ADT pattern は `LS3001` の focused rejection test で固定し、
  `lsharp-ir lower` 130 件、WasmGC probe 8 件、関連 clippy を実行する。

## Consequences

- ADT payload の concrete Ref と nested constructor pattern が record と同じ core WasmGC path で
  実行できる。
- `ref.null` を必要とする variant 間の nullable slot、literal pattern、parametric/GADT は別 slice
  として残る。`LEGACY-EXEC-01` は未完了であり、TODO の aggregate 要件は削除しない。
