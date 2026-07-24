# ADR: WasmGC module-link の typed funcref / GC type remap

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: 複数 IR module をリンクする際の型 index rebasing
- Related: `decisions-legacy-exec-wasmgc-typed-funcref-local.md`, `decisions-legacy-exec-wasmgc-captured-env-direct-call.md`, `decisions-legacy-exec-wasmgc-captured-env-let-alias.md`

## Context

Stage 3b は `RefFunc` / `CallRef` 命令の module-link remap を固定し、Stage 3g〜3i は
`TypedFuncRef` local と captured env struct を追加した。しかし `link_modules` は命令列だけを
clone/remap していたため、linked `Function` の params/result/locals と `GcTypeDef` field が
module-local の `Ref` / `TypedFuncRef` index を保持したままだった。

## Decision

- `IrType::Ref(index)` は `(module, gc_type_index)` remap、`IrType::TypedFuncRef(index)` は
  `(module, function_type_index)` remap を通す共通 helper で更新する。
- Function の params/result/locals と、GC struct field / array element type に同じ helper を適用する。
- 命令内の `RefFunc` / `CallRef`、import dedup、function type の順序は既存の linker 契約を維持する。
- global 型、module graph の cache、component/native target parity はこの slice の対象に含めず、
  未検証の境界として残す。

## Evidence

- RED: `test_link_funcref_rebases_typed_local_and_gc_field_types` は実装前に linked function の
  `TypedFuncRef(2)` が `TypedFuncRef(4)` へ移らず失敗した。
- GREEN: 同テストは 2 module の function params/result/locals、import signature、GC struct fields の
  `Ref`/`TypedFuncRef` が、GC prefix と import prefix を含む linked index へ変換されることを確認する。
- `test_link_funcref_rebases_array_element_type` は `GcTypeKind::Array` の element
  `TypedFuncRef` も linked function type index へ変換されることを確認する。
- Regression: `cargo test -p lsharp-ir linker_tests`（7 tests）、focused linker test、clippy、rustfmt、docs
  audit を通過し、命令 remap の既存 `RefFunc` / `CallRef` 契約を維持した。lib 全体 234 tests も実行し
  233 件は通過したが、既存の `incremental_compile_tests::test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds`
  が Formatter の `format-expr` undefined 診断で失敗しており、本 slice 外の既知 failure として残す。

## Residual risk

この決定は IR linker の型 index 境界に限定された verified partial slice である。recursive GC type
group の cross-module emission、global/module cache remap、captured closure の returned/general
higher-order path、WASI/component、Mac Apple Silicon / Linux x86_64 native stage0 と selfhost parity は
未完了で、`LEGACY-EXEC-01` の aggregate 完了条件には到達していない。
