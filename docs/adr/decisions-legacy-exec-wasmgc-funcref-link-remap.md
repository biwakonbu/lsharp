# ADR: WasmGC module-link funcref index/type remap

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp-ir::link_modules` の `RefFunc` / `CallRef`

## Context

WasmGC emitter の typed funcref capability は単一 IR module で成立しても、複数 module を
link すると import dedup、GC type の連結、user function の連結で index 空間が変わる。
`RefFunc` を古い function index のまま残すと別関数を参照し、`CallRef` を古い type index の
まま残すと異なる signature の function type を指定する。

## Decision

- `RefFunc` は `Call` / `FuncIdx` と同じ import/user function index 空間として扱い、既存の
  `import_remap` / `func_remap` を共有する。
- `CallRef` は IR type section の `GC type → import function type → user function type` の順序を
  local module から linked module へ写像する `function_type_remap` を通す。
- import function type は deduplicated import index を使い、user function type は linked function
  の安定順を使う。未知の type index は暗黙に補正せず、そのまま後段 validation へ渡す。
- closure lowering や synthetic backend import の最終 mapping はこの slice の責務外とする。

## Evidence

- RED: `test_link_funcref_rebases_function_and_type_indices` は user `RefFunc` が module-local
  index `1` のまま残るため失敗した。
- GREEN: 同テストは 2 module の異なる GC type/import/user function を連結し、import/user
  `RefFunc` と import/user `CallRef` の全 remap を確認する。
- Focused gate: `cargo test -p lsharp-ir linker_tests::test_link_funcref_rebases_function_and_type_indices
  -- --test-threads=1`。

## Residual risk

closure env lowering はまだ `CallIndirect` / linear-memory 表現であり、WasmGC の実 closure E2E、
synthetic `print-string` / Component output type の backend 側 mapping、trait vtable、Mac Apple
Silicon / Linux x86_64 native stage0 と selfhost parity は未検証である。
