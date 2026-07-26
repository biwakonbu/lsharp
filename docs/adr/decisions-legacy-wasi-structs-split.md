# ADR: WASI linear-memory struct emission split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi/structs.rs`
- Related: `I-01`, `I-08`, `decisions-legacy-module-graph-scc-split.md`

## Context

`wasi.rs` は WASI runtime と linear-memory backend の code emission を同じ production
file に保持している。`StructNew` / `StructGet` / `StructSet` の field validation、scratch
local layout、memory load/store emission は独立した責務だが、GC runtime や file/argv helper
の変更と同じ差分単位になっていた。

## Decision

- `WasiStructScratch`、struct field count validation、linear-memory struct instruction
  emission を `wasi/structs.rs` へ移動する。
- Preview1 / Preview2 の caller は `structs` module の helper を使い、local index layout、
  allocator call、field offsets、out-of-bounds diagnostics、未対応 instruction の
  `Ok(false)` 契約を維持する。
- helper は `pub(super)` に留め、公開 API・Wasm function ordering・runtime ABI は変更しない。
- 空 module の scratch minimum と既存 record field-access runtime を seam/E2E で固定する。

## Evidence

- RED: 空の `structs` module に対する seam test が module 不在で失敗。
- GREEN: `wasi::structs_tests::structs_helper_reserves_one_scratch_field_for_empty_modules`
  （1 passed）。
- `cargo test -p lsharp-wasm wasi:: --lib`: 40 passed / 1 existing root-lifetime failure。
- `cargo test -p lsharp-wasm --test e2e test_e2e_record_field_access_compile -- --nocapture`:
  1 passed。
- `cargo test -p lsharp-wasm --lib`: 99 passed / 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`、workspace check、変更対象 Rust 2024
  rustfmt、`git diff --check`、`bash scripts/audit_docs.sh` が pass。

## Consequences

struct lowering を runtime helper と独立してレビュー・再実行でき、`wasi.rs` は 3237 行から
3133 行へ縮小した。この slice は WasmGC/linear-memory parity 全体、native/selfhost ABI、
dynamic memory layout、Mac/Linux stage0、I-01 / I-08 aggregate の完了を意味しない。
