# ADR: IR linker production seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/linker.rs`, `crates/lsharp-ir/src/lib_tests.rs`

## Context

`lsharp-ir/src/lib.rs` は IR model、linker、incremental compile orchestration を同じ parent に保持していた。model と instruction の seam を分離した後も、`link_modules` と import/function/GC/function-type index の remap helpers が parent に残っていた。linker の public path と WasmGC の index semantics を変えずに、production ownership を分ける必要がある。

## Decision

`link_modules`、`remap_ir_type`、`remap_gc_type_definition`、`remap_instruction_with_imports` を `linker.rs` に移動する。`lib.rs` は `mod linker` と `pub use linker::link_modules` で既存の `lsharp_ir::link_modules` を再公開する。helper は linker module 内に閉じ、model/instruction の型は parent の public re-export を介して参照する。

## Evidence

- RED: `mod linker` と `pub use linker::link_modules` を先に追加し、`cargo test -p lsharp-ir --lib test_link_modules_public_seam_preserves_import_rebase_contract` で `E0583`（`linker.rs` 不在）を確認。
- GREEN: linker body を移動後、同 focused test は pass。移動元の origin/main block と `linker.rs` body の比較も一致した。
- `cargo test -p lsharp-ir --lib linker -- --nocapture`: 7 pass。
- `cargo test -p lsharp-ir --lib -- --nocapture`: 286 pass / 1 fail。失敗は origin/main から継続する `selfhost/src/Tools/Validation/IntentSource.ls` の `vector-push-pair-rooted-v3` 未定義であり、本分割の変更起因ではない。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、workspace `cargo check`、対象 Rust 2024 rustfmt、`git diff --check`、docs audit は pass。

## Consequences

linker の ownership が `linker.rs` に集約され、`lib.rs` は 2428 行から 2139 行へ縮小した。既存 public path と import deduplication、function/GC/function-type remap、instruction remap semantics は維持される。lowering、incremental orchestration、native/selfhost parity、I-01 / I-08 aggregate は未完了であり、`LEGACY-MAINT-01` は `[~]` のまま継続する。
