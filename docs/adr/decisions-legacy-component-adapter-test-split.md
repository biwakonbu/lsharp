# ADR: `lsharp-wasm/component_adapter.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/src/component_adapter.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`component_adapter.rs` は core Wasm の Component 化、WIT metadata、artifact の atomic 保存を
実装し、同じファイル末尾に artifact round-trip、Preview1 adapter、WasmGC boundary などを確認する
8 件の Wasmtime 回帰テストを保持していた。test-only fixture を分離すると、component adapter
production と Component/WIT runtime fixture の ownership/review 境界を明確にできる。

## Decision

- `componentize_core_module`、WIT metadata、artifact read/write の公開 API と production semantics は
  変更しない。
- `#[cfg(test)] mod tests` の helper と 8 件を
  `crates/lsharp-wasm/src/component_adapter_tests.rs` へ移動する。
- `component_adapter.rs` は `#[cfg(test)] #[path = "component_adapter_tests.rs"] mod tests;` で
  既存の `component_adapter::tests` namespace を維持する。
- artifact、Preview1 adapter、vendored HTTP WIT、canonical list<u8>、WasmGC bridge failure の
  fixture と assertion は変更しない。Wasm package の既存 root-lifetime/clippy failure は別タスクとして残す。

## Evidence

- 分離前後の `component_adapter::tests` focused gate: 8 passed。
- `component_adapter.rs` は 657 行から 377 行へ、`component_adapter_tests.rs` は 283 行となった。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-wasm`: 86 passed / 1 failed。失敗は既存
  `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap` の
  `RootLifetime { error: RootSetWithoutActiveSlot { ... } }` unwrap failure であり、今回の test-only 移動とは無関係。
- `cargo clippy -p lsharp-wasm --all-targets -- -D warnings` は既存 `wasi.rs`、
  `tests/native_cli_output.rs`、`tests/e2e/support.rs`、`tests/e2e/selfhost_native_stage_chain.rs` の
  lint debt で停止し、今回の component adapter 差分由来の warning はない。
- Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

component adapter production と Wasmtime/WIT fixture の ownership/review 境界が明確になり、8 件の
回帰テストを単独で再実行できる。他の Wasm production split、既存 root-lifetime/clippy baseline、
I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
