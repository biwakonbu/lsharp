# ADR: WASI tests core fixture split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/src/wasi_tests.rs`, `crates/lsharp-wasm/src/wasi_tests/core.rs`, `crates/lsharp-wasm/src/wasi_tests/preview2.rs`
- Related: `LEGACY-MAINT-01`, Issues `I-01` / `I-08`, `docs/development/planning/improvement-designs/imp-06-large-file-decomposition.md`

## Context

`wasi_tests.rs` は shared Wasm/WASI compile-run helpers、Preview1/core regression tests、Preview2 tests を一つの `#[cfg(test)] mod tests` に保持していた。Preview2 tests は既に `preview2.rs` へ移動済みだが、Preview1/core の test body が parent に残り、test-only file の責務境界が不明瞭だった。

## Decision

Preview1/core の test body と assertion helper を `wasi_tests/core.rs` へ移動する。親 `wasi_tests.rs` には shared `compile_wasi` / `compile_wasi_p2` / `run_wasi` helper と root-slot runner を残し、`include!` を同じ `tests` module 内で使う。これにより `wasi::tests::*` の namespace、private helper visibility、test names、Preview2 include path を変更しない。production code、WASI ABI、公開 API は変更しない。

## Evidence

- RED: `include!("wasi_tests/core.rs")` を先に追加し、child 未作成の `E0583` を `cargo test -p lsharp-wasm --lib --no-run` で確認。
- GREEN: `cargo test -p lsharp-wasm wasi::tests::test_wasi_test_module_keeps_shared_preview1_fixture -- --nocapture` は pass。
- Full lib: `cargo test -p lsharp-wasm --lib` は 114 pass / 1 existing `RootLifetime::RootSetWithoutActiveSlot` failure（分割前からの既知 failure boundary）。
- Static gates: `cargo clippy -p lsharp-wasm --lib -- -D warnings`、`cargo check --workspace`、対象 Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`。

## Consequences

`wasi_tests.rs` は 568 行から 94 行、core child は 474 行となり、shared fixture と test suite の ownership が分離される。test-only の構造変更なので production runtime/API の変更はない。WASI/native/selfhost parity、Mac Apple Silicon / Linux x86_64 stage0、I-01 / I-08 aggregate は未完了であり、TODO の `[~]` を維持する。
