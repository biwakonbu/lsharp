# ADR: WASI compiler-world Code Section の責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi/compiler_world.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md)

## Context

`compiler_world.rs` は 761 行の単一 file で、WASI の import・memory・global・table・export・data
section の組み立てと、runtime helper・user function・`_start` の Code Section 生成を同じ
`emit_wasm_wasi_with_options` に保持していた。Code Section の変更が module layout と混ざり、
ABI を変えない責務分離のレビューが難しかった。

## Decision

- Code Section の helper/user function/entrypoint emission を
  `wasi/compiler_world/code.rs` の `emit_code_section` へ移動する。
- parent には module section assembly と index 計算を残し、private な
  `WasiCodegenIndices` を child へ渡す context seam とする。
- `emit_wasm_wasi_with_options` の API、section order、function/table/memory/global/export/data
  layout、runtime imports/ABI、user function body、`__proc_exit_with_collect`、`_start`、optional
  component runner の semantics は変更しない。

## Evidence

- RED: `mod code;` を追加した時点で child 不在の `E0583` を確認。
- GREEN: `cargo test -p lsharp-wasm compiler_world_module_emits_empty_wasi_core_module -- --nocapture` — 1 passed。
- WASI focused: `RUST_MIN_STACK=33554432 cargo test -p lsharp-wasm wasi:: --lib -- --nocapture` — 48 passed、既存の `test_root_set_invalid_slot_records_failure_ledger_before_trap` 1 failure。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`、`cargo check --workspace`、対象 Rust 2024 `rustfmt --check`、`git diff --check`、`bash scripts/audit_docs.sh` が pass。
- parent は 761 行から 693 行へ縮小し、Code Section child は 166 行となった。

## Consequences

Code Section の変更範囲を module layout から分離してレビューできる。公開 API、Wasm section order、
runtime ABI は維持される。all-targets clippy に残る既存 test warnings、root-lifetime の既存 failure、
full Rust/native parity、I-01 / I-08 aggregate はこの partial slice では完了扱いにしない。
