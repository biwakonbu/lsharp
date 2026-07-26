# ADR: `lsharp-wasm/wasi_tests.rs` Preview2 test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi_tests.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`wasi_tests.rs` は Preview1 の compile/runtime 回帰、WASI errno 検査、root failure ledger、
Preview2/Component の compile・stdout・stdin/argv・filesystem 回帰を同じ `wasi::tests`
module に保持していた。Preview2/Component の 5 件は WIT/component runner の境界でまとまっており、
別ファイルへ移しても Preview1 の helper と同じ test module scope を共有できる。

## Decision

- Preview2/Component の 5 件を `crates/lsharp-wasm/src/wasi_tests/preview2.rs` へ移動する。
- `wasi_tests.rs` は `include!("wasi_tests/preview2.rs")` を既存 `wasi::tests` module 内で評価し、
  `compile_wasi_p2` と private helper の利用、既存 test path を維持する。
- Preview1/Component の runtime semantics、WIT validation、stdin/argv、large stdout、filesystem
  roundtrip の assertion は変更しない。
- Wasm production、native/selfhost parity、既知の root-lifetime failure はこの test-only slice の
  範囲外として残す。

## Evidence

- RED: `wasi_tests/preview2.rs` を参照する include を先に追加し、ファイル不在の `E0583` を確認した。
- Preview2 focused gate: `cargo test -q -p lsharp-wasm wasi::tests::test_emit_wasm_wasi_p2 --lib -- --nocapture` は 5 passed。
- `wasi::tests` 全 28 件は 27 passed / 1 failed。失敗は既存
  `test_root_set_invalid_slot_records_failure_ledger_before_trap` の
  `RootLifetime::RootSetWithoutActiveSlot` unwrap failure で、test 移動との差分外である。
- `wasi_tests.rs` は 647 行から 562 行へ縮小し、Preview2 test file は 86 行。公開 API と
  `wasi::tests::*` の test path は維持した。
- `cargo clippy -q -p lsharp-wasm --lib -- -D warnings`、`cargo check --workspace --quiet`、
  対象 files の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

Preview1 の helper/assertion と Preview2/Component runtime regression の ownership/review 境界が
明確になり、Component 側の 5 件だけを focused に再実行できる。WASI production decomposition、
全 backend/native/selfhost parity、I-01 / I-08 aggregate は未完了である。
