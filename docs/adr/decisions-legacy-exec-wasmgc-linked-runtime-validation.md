# ADR: linked WasmGC module の emitter/Wasmtime validation

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `link_modules` 後の typed funcref/GC type を含む WasmGC artifact
- Related: `decisions-legacy-exec-wasmgc-module-link-typed-types.md`

## Context

Stage 3j は module-local な `Ref` / `TypedFuncRef` を linked IR の params/result/locals/import/GC
fields へ remap した。IR の index assertion だけでは、recursive type group の Wasm emitter が生成する
実際の type section、function section、runtime module の validation まで保証できない。

## Decision

- 2 module を `link_modules` で結合し、left/right の GC type と function type prefix を持つ linked IR を
  WasmGC emitter に渡す。
- emitter は typed funcref を含むため GC/function types を recursive type group として出力し、
  linked `Ref` / `TypedFuncRef` index をそのまま concrete heap/function type へ変換する。
- Wasmtime 29 で WasmGC、reference types、typed function references を有効化して validate、
  instantiate、exported `right-main` の `42` 実行までを同一 test で固定する。
- component/WASI/native stage0 と Linux x86_64 はこの slice の対象外で、Mac host 上の runtime evidence
  としてのみ記録する。

## Evidence

- `wasm_gc_emitter_validates_linked_typed_funcref_and_gc_types` は linked function signatures と
  GC fields の index を IR で確認する。
- 同テストは linked Wasm bytes の Wasmtime validation/instantiate と `right-main == 42` を確認する。
- WasmGC probe 101 件、focused test、clippy、rustfmt を通過した。

## Residual risk

これは Mac Apple Silicon 上の linked core module validation/runtime に限定された verified partial slice
である。component/WASI bridge、module graph/cache/global remap、Linux x86_64 native/selfhost parity、
captured closure の returned/general higher-order path、GC rooting は未完了で、`LEGACY-EXEC-01` の
aggregate 完了条件には到達していない。
