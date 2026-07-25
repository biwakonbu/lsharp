# ADR: lower do production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-let-split.md`

## Context

`lower/expr.rs` の `Expr::Do` production は、逐次式の評価、中間値の `Drop`、空 do の unit
値生成を `lower_expr` の match arm に抱えていた。sequence control-flow と他の expression
production が混在し、変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Do` の lowering を `lower/expr/do_expr.rs`（21 行）の `lower_do` へ移動する。
- 親 `expr.rs` は expression list を helper へ渡すだけにし、評価順序、中間値破棄、空 sequence の
  unit、error semantics を維持する。
- do module seam test で、複数式の value → `Drop` → 最終 value の順序を固定する。

## Evidence

- RED: seam test は `lower_do` 未定義で `no method named lower_do` として失敗。
- GREEN: seam test が中間式の `Drop` と最終式の順序を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（159 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは do production の責務分離だけを扱う。全 control-flow/backend/native/runtime parity、
selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
