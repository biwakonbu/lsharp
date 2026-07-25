# ADR: WASI root helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-wasi-hash-split.md`

## Context

`wasi.rs` は WASI preview1 / preview2、GC runtime、I/O helper を一つの production
module に持つ。`root_push` / `root_pop` / `root_set` は root table の容量拡張、
`memory.grow`、slot bounds、failure ledger、slot の load/store を担う独立した code-emission
責務だが、WASI entrypoint と同じ parent にあり、GC-safe-point 周辺の変更衝突単位を増やしていた。

## Decision

- `emit_root_push_func`、`emit_root_pop_func`、`emit_root_set_func` を
  `crates/lsharp-wasm/src/wasi/root.rs`（225 行）へ移動する。
- Preview1/Preview2 の helper registration は `root` module 経由にし、function ordering、
  import/index、linear-memory ABI、root slot の tagged i64 contract は維持する。
- `root_push` の容量倍増・`memory.grow`・既存 table の `memory.copy`、`root_set` の
  out-of-range failure ledger と `unreachable`、`root_pop` の空 stack semantics は変更しない。
- `root_tests.rs` の module seam test で 3 helper body の登録を固定する。

## Evidence

- RED: 空の `root` module に対する seam test が 3 helper の unresolved import で失敗。
- GREEN: `cargo test -p lsharp-wasm --lib root_module_emits_push_pop_and_set_function_bodies -- --nocapture`
  （1 passed）。
- `cargo test -p lsharp-wasm --lib`（91 tests のうち 90 passed、既存の
  `RootLifetime::RootSetWithoutActiveSlot` failure 1 件）。
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは WASI root helper の責務分離だけを扱う。root stack の Rust/native selfhost parity、
dynamic memory layout、全公開 command、Mac Apple Silicon / Linux x86_64 native stage0 の
完了を意味しない。既存の `vector-push-pair-rooted-v3` selfhost fixture failure、root
lifetime checker の既知 failure、package-wide test-only lint debt は今回の差分外として残る。
