# ADR: tooling compile test suite 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-tooling/src/compile.rs`, `crates/lsharp-tooling/src/compile_tests_cache.rs`, `crates/lsharp-tooling/src/compile_tests_wasmgc_a.rs`, `crates/lsharp-tooling/src/compile_tests_wasmgc_b.rs`, `crates/lsharp-tooling/src/compile_tests_outputs.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`lsharp-tooling/src/compile.rs` は compile target/cache pipeline と 66 件の cache、WasmGC、native、
output 回帰テストを同じ約 2870 行のファイルへ保持していた。test-only fixture の変更が production
compile boundary と同じ diff に混ざり、レビュー範囲とファイルサイズを広げていた。

## Decision

- inline `compile::tests` の test body を cache、WasmGC A、WasmGC B、output の 4 fragment へ移動する。
- 親の `compile::tests` module 内で `include!` し、既存の test namespace、`super::*` による private helper access、
  compile/cache/WasmGC/native/output fixture ownership を維持する。
- production の compile target/backend、artifact cache、Wasm validation、native output、公開 API は変更しない。
- target/backend tag の stable text contract を回帰テストで明示する。

## Evidence

- RED: `include!("compile_tests_cache.rs")` を追加した child 不在状態で `E0583` を確認。
- GREEN: `test_compile_target_and_backend_tags_are_stable` と既存 compile suite が pass。
- Package: `cargo test -p lsharp-tooling -- --nocapture` — unit 135、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-tooling --all-targets -- -D warnings`、`cargo check --workspace`（専用 target）、対象
  Rust 2024 `rustfmt --check`、`git diff --check` が pass。
- parent は 2870 行から 492 行、fragment は 486 / 544 / 708 / 635 行となった。

## Consequences

tooling compile production と test-only fixture を独立してレビューでき、parent は 500 行未満の production
boundary となる。既存の test namespace と compile semantics は維持される。tooling compile production の
追加分割、selfhost/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
