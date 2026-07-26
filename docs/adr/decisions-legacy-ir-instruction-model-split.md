# ADR: lsharp-ir public Instruction/IrType model 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/instruction.rs`, `crates/lsharp-ir/src/lib_tests.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-ir/src/lib.rs` は Module/Function/GC model、public `IrType`/`Instruction` model、
linking、incremental compile pipeline を一つの parent に保持していた。`IrType` と `Instruction` は
public model/opcode contract であり、lowering/linking orchestration と独立にレビューできる seam である。

## Decision

- `IrType` と `Instruction` の定義および Display 実装を `instruction.rs` へ移動する。
- parent では `mod instruction; pub use instruction::{Instruction, IrType};` を使い、既存 public paths と internal type references を保つ。
- enum variants、Instruction→IrType references、Display strings、opcode representation は変更しない。
- `CallImport(7)` の display contract test を追加する。

## Evidence

- RED: `mod instruction;` を追加した child 不在状態で `E0583` を確認。
- GREEN: focused display test は pass。`cargo test -p lsharp-ir --lib` は 284 pass / 1 known existing `IntentSource.ls` failure。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2842 行から 2564 行、`instruction.rs` は 282 行となった。既存 `lsharp_ir::Instruction` / `IrType` public path と production semantics は維持した。

## Consequences

Public IR model/opcode ownership is independently reviewable while Module/linking/incremental orchestration remains in `lib.rs`. Remaining Module/Function/GC model, linker/lowering production decomposition, Rust/native parity, and I-01/I-08 aggregate are incomplete.
