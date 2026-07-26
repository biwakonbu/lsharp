# ADR: IR compile/incremental orchestration seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-ir/src/lib.rs`, `crates/lsharp-ir/src/compile.rs`, `crates/lsharp-ir/src/compile_support.rs`, `crates/lsharp-ir/src/compile_pipeline.rs`, `crates/lsharp-ir/src/compile_entrypoints.rs`, `crates/lsharp-ir/src/compile_incremental.rs`, `crates/lsharp-ir/src/lib_tests/compile.rs`

## Context

`crates/lsharp-ir/src/lib.rs` は IR model、linker、compile surface の分割後も、
multi-file compile、SCC inference、segment reuse、incremental analysis/cache 更新を
同じ parent に保持していた。`LEGACY-MAINT-01` の 500〜800 行規約を満たしつつ、
`lsharp_ir::compile_multi_file` などの公開入口と既存の private test seam を変えずに
production ownership を分離する必要がある。

## Decision

compile/incremental orchestration を `compile` module の include seam に分割する。

- `compile_support.rs`: parse/cache helper、segment layout/link patch、shared lowering state
- `compile_pipeline.rs`: merged/modular lowering、SCC surface inference、multi-file pipeline
- `compile_entrypoints.rs`: `compile_multi_file` / `compile_multi_file_with_cache` と mode entry
- `compile_incremental.rs`: incremental SCC compile、source override analysis、cache compile

`compile.rs` は imports と4 fragmentの順序付き `include!` だけを持つ。親 `lib.rs` は
既存の公開入口を `pub use compile::{...}` で再公開し、private helper は `pub(super)` と
crate 内 test 用 re-export に限定する。ロジック、エラー文字列、cache key、SCC 順序、
IR segment の再利用 semantics は変更しない。

## Evidence

- RED: `mod compile` と公開 re-export、compile seam test を先に追加し、child file がない
  状態で `cargo test -p lsharp-ir --lib compile_module_seam --no-default-features` を実行して
  `E0583`（`compile.rs` 不在）を確認した。
- GREEN: body を移動後、`compile_tests::test_compile_module_seam_preserves_full_and_cached_entrypoints`
  は full compile と cache compile の dump/cache 件数一致を確認した。
- `cargo test -p lsharp-ir --lib`: 289 tests passed。
- `cargo test -p lsharp-ir --quiet`: 289 tests passed。
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、`cargo check --workspace --quiet`、
  対象 Rust 2024 files の `rustfmt --check`、`git diff --check` は pass。
- workspace 全体の `cargo fmt --all -- --check` は本変更外の既存 files にも大量の formatting
  差分を報告したため、対象 files の rustfmt gate と分離して扱う。

## Consequences

`lib.rs` は 2016 行から 80 行へ縮小し、compile fragments は 524 / 597 / 122 / 697 行に
収まった。既存の public path と incremental/cache semantics は維持される。
これは `LEGACY-MAINT-01` の verified slice であり、`[~]` は継続する。native/selfhost
parity、full compiler/runtime gate、残る large production files は別タスクとして残す。
