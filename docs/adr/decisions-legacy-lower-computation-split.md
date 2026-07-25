# ADR: lower computation production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-match-split.md`

## Context

`lower/expr.rs` の `Expr::Computation` production は、WasmGC の未対応境界、builder 情報の
解決、`let!` / `do!` / `return` / 通常式の脱糖、local binding と intermediate value の
破棄を一つの `lower_expr` match arm に抱えていた。computation の step semantics と他の
expression production が混在し、変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Computation` の lowering を `lower/expr/computation.rs`（83 行）の
  `lower_computation` へ移動する。
- 親 `expr.rs` は span、builder 名、steps を helper へ渡すだけにし、WasmGC rejection、
  builder return call、pattern local binding、step ごとの Drop、error/span semantics を維持する。
- computation module seam test で、return 式の値 lowering と builder return function call の
  順序を固定する。

## Evidence

- RED: seam test は `lower_computation` 未定義で `no method named lower_computation` として失敗。
- GREEN: seam test が return 値を積んだ後に builder return function を call することを確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（157 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは computation production の責務分離だけを扱う。computation expression の全 backend
parity、native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
