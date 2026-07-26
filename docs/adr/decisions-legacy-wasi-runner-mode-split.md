# ADR: WASI runner の Preview1 / Preview2 mode seam 分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/src/wasi_runner.rs`,
  `crates/lsharp-wasm/src/wasi_runner/preview1.rs`,
  `crates/lsharp-wasm/src/wasi_runner/preview2.rs`,
  `crates/lsharp-wasm/src/wasi_runner_tests.rs`
- Related: [LEGACY-MAINT-01](../../TODO.md),
  [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`wasi_runner.rs` は Preview1 core Wasm、Preview2 Component Model、mode routing、stdout/exit
共通処理を一つの module に保持していた。今後の runtime/ABI 修正で両 mode の差分が同じ
parent に混在すると、片方の変更がもう片方の public API や diagnostics を壊しやすい。

## Decision

Preview1 の実行関数群を `wasi_runner/preview1.rs`、Preview2 の Component Model 実行関数群を
`wasi_runner/preview2.rs` へ分離する。parent は `WasiMode`、mode routing、共通 engine/stdout/
exit/diagnostic helper と、既存 public path の `pub use` に限定する。Preview1/Preview2 の
function names、引数、戻り値、error text、stdin/argv/directory semantics、component run export
探索順は変更しない。

## Evidence

- RED: parent に `preview1` / `preview2` module seam を先に宣言し、child file 不在の `E0583` を
  `cargo check -p lsharp-wasm --lib` で確認した。
- GREEN: `cargo test -p lsharp-wasm --lib wasi_runner::tests -- --nocapture` は 26 tests pass。
  mode-specific invalid artifact diagnostics、Preview1/Preview2 runtime、stdin/argv、component
  export fallback、capacity/root diagnostics を含む。
- `LSHARP_EMBED_COMPONENT_PATH=... cargo check -p lsharp-wasm --lib`、
  `cargo clippy -p lsharp-wasm --lib -- -D warnings`、対象 Rust 2024 rustfmt、`git diff --check`
  は pass。
- `cargo test -p lsharp-wasm --lib` は 110 pass / 1 existing `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap` failure。これは今回の
  runner split と無関係な origin/main 由来 root-lifetime failure として切り分けた。all-tests
  clippy は既存 `native_cli_output.rs` / `wasi_tests.rs` の closure lint で阻害される。

## Consequences

mode ごとの runtime implementation を独立して変更・検証でき、parent の責務は 581 行から
196 行へ縮小した。public `lsharp_wasm::wasi_runner::*` path と observable behavior は維持した。
これは WASI runner の verified partial decomposition であり、root-lifetime failure、全 native/
selfhost parity、両 supported target の runtime evidence、`LEGACY-MAINT-01` aggregate の完了を
意味しない。
