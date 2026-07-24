# ADR: WasmGC compile output の atomic artifact 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-24
- Scope: `lsharp_tooling::compile::compile_file_with_backend` / `CompileBackend::WasmGc`

## Context

Linear backend の compile output は共通の Wasm artifact writer へ移行済みだったが、WasmGC
backend の `WebWasm` output だけが destination を `std::fs::write` で直接 truncate していた。
compile backend を切り替えたときだけ保存境界が変わると、失敗した生成物が既存 artifact を壊す
リスクと診断契約の不一致が残る。

## Decision

- WasmGC の `WebWasm` output も `write_compile_artifact` を利用し、共通の一時 path → `rename`
  境界を通す。
- 置換失敗時は temporary artifact を cleanup し、既存 destination は変更しない。
- WasmGC の target 制約、module validation、runtime semantics は変更しない。

## Evidence

- RED: `test_compile_file_wasmgc_backend_uses_atomic_artifact_boundary` は変更前に直接
  `Is a directory` 診断で失敗した。
- GREEN: 同テストで `Wasm artifact の置換` 診断、既存 destination 保持、一時 artifact 残留なしを
  確認した。
- WasmGC compile/runtime focused tests と WasmGC probe regression は別途実行する。

## Residual risk

durable fsync、source commit/fingerprint manifest、Linux x86_64 native backend、selfhost stage0
release/rollback、external release bundle は未完了である。atomic rename の成功を WasmGC 全機能
Rust-free 完了や release readiness へ拡大解釈しない。
