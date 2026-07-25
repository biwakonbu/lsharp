# ADR: `lsharp-wasm/wasi_runner.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/src/wasi_runner.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`wasi_runner.rs` は Preview1/Preview2 の Wasm 実行、stdout/stderr capture、directory binding、
runtime failure classification を実装し、同じファイル末尾に 25 件の Wasmtime runtime 回帰テストと
compile/runtime fixture helper を保持していた。test-only fixture を分離すると、runner production と
WASI/Component runtime fixture の ownership/review 境界を明確にできる。

## Decision

- `run_wasm_*`、WASI/Component mode dispatch、runtime failure classification の公開 API と production
  semantics は変更しない。
- `#[cfg(test)] mod tests` の 25 件と helper を
  `crates/lsharp-wasm/src/wasi_runner_tests.rs` へ移動する。
- `wasi_runner.rs` は `#[cfg(test)] #[path = "wasi_runner_tests.rs"] mod tests;` で既存の
  `wasi_runner::tests` namespace を維持する。
- Preview1/Preview2 execution、stdin/stdout soak、directory binding、failure classification の fixture
  と assertion は変更しない。Wasm package の既存 root-lifetime/clippy failure は別タスクとして残す。

## Evidence

- 分離前後の `wasi_runner::tests` focused gate: 25 passed。
- `wasi_runner.rs` は 1033 行から 581 行へ、`wasi_runner_tests.rs` は 450 行となった。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-wasm`: 86 passed / 1 failed。失敗は既存
  `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap` の
  `RootLifetime { error: RootSetWithoutActiveSlot { ... } }` unwrap failure であり、今回の test-only 移動とは無関係。
- `cargo clippy -p lsharp-wasm --all-targets -- -D warnings` は既存 `wasi.rs`、
  `tests/e2e/support.rs`、`tests/e2e/selfhost_native_stage_chain.rs` の lint debt で停止し、
  今回の wasi runner 差分由来の warning はない。
- Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

WASI/Component runner production と runtime regression fixture の ownership/review 境界が明確になり、
25 件の回帰テストを単独で再実行できる。既存 root-lifetime/clippy baseline、他の Wasm production split、
I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
