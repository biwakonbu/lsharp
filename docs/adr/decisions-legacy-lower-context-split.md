# ADR: lower function context split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/mod.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-state-split.md`

## Context

`lower/mod.rs` は `Lower` の program-state preparation、`FuncCtx`、共通 IR helper を
一つの module に抱えていた。program-state preparation の分離後も、関数 lowering の
context（local allocator、binding restore、instruction buffer）が親 module に残り、
責務境界と再利用単位が曖昧だった。

## Decision

- `FuncCtx` とその local allocator / binding restore / instruction emission を
  `lower/context.rs`（85 行）へ移す。
- `lower` module から `FuncCtx` を `pub(crate)` re-export し、`decl` / `expr` /
  `pattern` の既存 import と lowering API を維持する。
- compiler temporary (`_` prefix) の fresh allocation、named local の reuse、
  scoped binding の restore、`local_types` の更新 semantics は変更しない。
- context module seam test で named local の reuse と temporary local の fresh allocation
  を固定する。

## Evidence

- RED: `context` module 未作成時は
  `context_module_exposes_local_allocator` が `file not found for module context`
  で失敗。
- GREEN: seam test が named local の reuse (`0`, `0`) と temporary local の fresh
  allocation (`1`) を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（148 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`

## Boundary

これは lower の function context 責務分離だけを扱う。lower 全体の semantics parity、
native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
