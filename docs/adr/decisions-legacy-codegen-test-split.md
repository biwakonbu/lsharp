# ADR: `codegen.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/src/codegen.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`codegen.rs` は IR から Wasm への production code と、8 件の compile/runtime 回帰テストおよび Wasmtime stub harness を同じファイルに保持していた。test-only harness を分離すると、codegen production の変更と Wasm 実行 fixture の ownership/review 境界を明確にできる。

## Decision

- `emit_wasm`、`emit_instructions`、`CodegenError` の API と production semantics は変更しない。
- `#[cfg(test)] mod tests` の 8 件と helper を `crates/lsharp-wasm/src/codegen_tests.rs` へ移動する。
- `codegen.rs` は `#[cfg(test)] #[path = "codegen_tests.rs"] mod tests;` で既存の `codegen::tests` namespace を維持する。
- Wasm runtime stub の挙動、fixture、assertion は変更しない。parser/type/lowering や他の Wasm production split は別タスクとして残す。

## Evidence

- 分離前後の `codegen::tests` focused gate: 8 passed。
- `codegen.rs` は 520 行から 237 行へ、`codegen_tests.rs` は 279 行となった。
- 対象 files の rustfmt check、`git diff --check` は pass。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-wasm` は 86 passed / 1 failed。失敗は既存 `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap` の `RootLifetime` unwrap failure であり、codegen の test-only 移動とは無関係。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-wasm --all-targets -- -D warnings` は既存 `wasi.rs`、`native_cli_output.rs`、E2E の clippy warning で停止し、今回の差分由来の warning はない。

## Consequences

codegen production と Wasmtime compile/runtime fixture の ownership/review 境界が明確になり、8 件の回帰テストを単独で再実行できる。既存 Wasm package の root-lifetime failure と clippy baseline、parser/type/lowering および他の大規模 Rust file と I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
