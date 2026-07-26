# ADR: IR public module model の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/model.rs`, `crates/lsharp-ir/src/lib_tests.rs`

## Context

`lsharp-ir/src/lib.rs` は linker、incremental orchestration、公開 IR 型の責務が同居していた。前段の `Instruction` / `IrType` 分割後も、`Module` と関数・GC model 定義および `Module::dump` が parent に残り、production seam の ownership が曖昧だった。公開 API と IR の表示形式を変えずに、次の責務境界を作る必要がある。

## Decision

`Module`、`ImportFunc`、`GlobalDef`、`GcTypeDef`、`GcTypeKind`、`GcField`、`Function` と `Module::dump` を `model.rs` に移動する。`lib.rs` は `mod model` と `pub use model::{...}` で既存の `lsharp_ir::Module` などの public path を再公開し、linker / lowering が参照する型名と field/variant は変更しない。

## Evidence

- RED: `mod model;` を追加した状態で `cargo test -p lsharp-ir --lib test_module_dump_preserves_public_model_display_contract` を実行し、`E0583`（`model.rs` 不在）を確認。
- GREEN: `model.rs` 追加後の同 focused test は pass。
- `cargo test -p lsharp-ir --lib -- --nocapture`: 285 pass / 1 fail。失敗は origin/main から継続する `selfhost/src/Tools/Validation/IntentSource.ls` の `vector-push-pair-rooted-v3` 未定義であり、本分割の変更起因ではない。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、workspace `cargo check`、対象 Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

モデル定義と表示実装の ownership が `model.rs` に集約され、`lib.rs` の production parent は 2428 行へ縮小した。既存 public path、IR model の構造、`Module::dump` 出力は維持される。一方、linker / lowering / incremental orchestration の分割、native/selfhost parity、I-01 / I-08 の完了条件は未達であり、`LEGACY-MAINT-01` は `[~]` のまま継続する。
