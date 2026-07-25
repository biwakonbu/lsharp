# ADR: lower annotation production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-if-split.md`

## Context

`lower/expr.rs` の `Expr::Ann` production は、型注釈を無視して内側の式を lowering する
責務を `lower_expr` の match arm に抱えていた。annotation の境界と通常の expression
dispatch が混在し、変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Ann` の lowering を `lower/expr/ann_expr.rs`（10 行）の `lower_ann` へ移動する。
- 親 `expr.rs` は内側の式を helper へ渡すだけにし、型注釈を無視する既存 semantics、評価順序、
  error propagation を維持する。
- annotation module seam test で、注釈付き式の内側 expression lowering を固定する。

## Evidence

- RED: seam test は `lower_ann` 未定義で `no method named lower_ann` として失敗。
- GREEN: seam test が内側の literal lowering を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（161 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは annotation production の責務分離だけを扱う。型注釈の全 backend/native/runtime parity、
selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
