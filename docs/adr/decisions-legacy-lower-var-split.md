# ADR: lower variable lookup production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-quote-split.md`

## Context

`lower/expr.rs` の `Expr::Var` production は、local binding、引数なし関数/ADT constructor、
lambda-lifted function の lookup と undefined diagnostic を一つの match arm に抱えていた。
名前解決の複数境界と通常の expression dispatch が混在し、変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Var` の lowering を `lower/expr/var_expr.rs`（30 行）の `lower_var` へ移動する。
- 親 `expr.rs` は source span と name を helper へ渡すだけにし、lookup の優先順位、Call/LocalGet
  命令、undefined error の name/span、error semantics を維持する。
- var module seam tests で local lookup と undefined diagnostic を固定する。

## Evidence

- RED: seam tests は `lower_var` 未定義で `no method named lower_var` として失敗。
- GREEN: seam tests が local `LocalGet` と undefined name/span を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（164 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは variable lookup production の責務分離だけを扱う。全 backend/native/runtime parity、
selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
