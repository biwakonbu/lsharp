# ADR: lower quote unsupported boundary split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-ann-split.md`

## Context

`lower/expr.rs` の `Expr::Quote` / `Expr::Unquote` / `Expr::UnquoteSplice` production は、
macro expansion 後には残らない構文を明示的な `Unsupported` 診断で止める境界を match arm に
抱えていた。未対応機能の拒否契約と通常の expression dispatch が混在し、変更衝突と review
単位を増やしていた。

## Decision

- Quote 系 expression の lowering を `lower/expr/quote_expr.rs`（13 行）の `lower_quote` へ
  移動する。
- 親 `expr.rs` は source span を helper へ渡すだけにし、diagnostic message、source span、
  `Unsupported` error semantics を維持する。
- quote module seam test で、未対応境界が明示診断として返ることを固定する。

## Evidence

- RED: seam test は `lower_quote` 未定義で `no method named lower_quote` として失敗。
- GREEN: seam test が Unsupported message と source span を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（162 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは quote 系 production の明示的な拒否境界の責務分離だけを扱う。macro expansion、
全 backend/native/runtime parity、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
