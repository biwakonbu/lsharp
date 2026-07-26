# ADR: WasmGC component-output writer / fd adapter 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasmgc_runner_component_output.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`wasmgc_runner_component_output.rs` は canonical `wasmgc-output` core/Component runner、Preview2
stdout bridge と、`std::io::Write` / WASI `fd_write` への出力 adapter を同じ module に保持していた。
writer の partial-write、flush、zero/over-report/errno の fail-closed 契約は独立した責務軸であり、
Component/WASI orchestration と分離して review できる。

## Decision

- `run_wasm_wasmgc_component_output_to_writer`、
  `run_wasm_wasmgc_component_output_to_fd_write`、`ComponentOutputFdWriteAdapter`、
  `run_wasm_wasmgc_to_writer` を `wasmgc_runner_output_writer.rs` へ移動する。
- 親 module は child の public functions を再 export し、既存の
  `wasmgc_runner::*` / `component_output::*` API path を維持する。
- `Write::write_all` / `flush` の順序、partial-write 再試行、WASI errno、zero/over-report の
  fail-closed behavior は変更しない。
- WasmGC language/native/selfhost parity と advanced runtime handoff はこの maintenance slice の
  範囲外として TODO に残す。

## Evidence

- RED: `mod output_writer` を先に宣言し、child file 不在の `E0583` を確認した。
- Existing runner seam: `component_output_module_decodes_cli_exit_results` は 1 passed。
- `cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed。
- `wasmgc_runner_component_output.rs` は 642 行から 543 行へ、child は 112 行となった。
- `cargo test -q -p lsharp-wasm --lib` は 109 passed / 1 failed。唯一の失敗は既存
  `wasi::tests::test_root_set_invalid_slot_records_failure_ledger_before_trap` の
  `RootLifetime::RootSetWithoutActiveSlot` unwrap failure であり、今回の writer 分離とは無関係。
- Preview2 file-roundtrip fixture の一時 directory は process id を含む名前へ改め、他セッションの
  同名 `/tmp` fixture との競合を避けた。この test-only hygiene は writer/runtime semantics を変更しない。
- `cargo clippy -q -p lsharp-wasm --lib -- -D warnings`、`cargo check --workspace --quiet`、
  対象 files の Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

writer/fd adapter の I/O 契約を単独で確認でき、Component/Preview2 orchestration の責務境界が
明確になった。public runner paths と runtime semantics は維持される。WasmGC の全 parity、
advanced runtime、I-01 / I-08 aggregate は未完了である。
