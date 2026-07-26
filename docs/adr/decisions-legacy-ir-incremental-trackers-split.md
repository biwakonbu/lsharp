# ADR: lsharp-ir incremental telemetry tracker 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/incremental_trackers.rs`, `crates/lsharp-ir/src/lib_tests/incremental_compile.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-ir/src/lib.rs` の incremental compile/analysis pipeline は、parse/type-infer/SCC/lower/link
の test-only telemetry thread-local と tracker guard も保持していた。これらは production IR lowering/linking
semantics ではなく、incremental regression tests の instrumentation boundary である。

## Decision

- parse/type-infer/SCC merged-fast-path/lower/module-segment/link/cache-hit の thread-local counters と tracker guard を `incremental_trackers.rs` へ移動する。
- parent では `#[cfg(test)] include!("incremental_trackers.rs")` を使い、`lib_tests` が参照する private names と `note_incremental_*` path を同一 module namespace に保つ。
- enable/count/reset/drop semantics、thread-local isolation、incremental production path は変更しない。
- parse tracker reset contract test を追加する。

## Evidence

- RED: `include!("incremental_trackers.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: parse tracker reset test、incremental compile focused 11件、incremental analysis focused 15件、`cargo test -p lsharp-ir --lib` 284件が pass。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、専用 target の `cargo check --workspace`、対象 Rust 2024 `rustfmt --check`、`git diff --check`、docs audit が pass。
- parent は 3080 行から 2842 行、`incremental_trackers.rs` は 240 行となった。今回の変更で production semantics は変更していない。

## Consequences

Incremental instrumentation ownership is independently reviewable and `lib.rs` production pipeline remains behaviorally unchanged. Remaining compiler/linking production decomposition, lower/type representation, Rust/native parity, I-01/I-08 aggregate are incomplete.
