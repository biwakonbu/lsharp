# ADR: lower match production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-record-split.md`

## Context

`lower/expr.rs` の `Expr::Match` production は、scrutinee の評価と型名からの IR 型選択、
temporary local への保存、pattern arm の if-else chain への委譲を `lower_expr` の match arm
に抱えていた。match の control-flow preparation と他の expression production が同居し、
変更衝突と review 単位を増やしていた。

## Decision

- `Expr::Match` の scrutinee localization と arm dispatch を
  `lower/expr/match_expr.rs`（31 行）の `lower_match_expr` へ移動する。
- 親 `expr.rs` は scrutinee と arms を helper へ渡すだけにし、評価順序、型名由来の
  `IrType` 選択、`_match` local、`lower_match_arms` の既存 control-flow semantics を維持する。
- match module seam test で、scrutinee が local に保存されてから wildcard arm body が lowering
  される contract を固定する。

## Evidence

- RED: seam test は `lower_match_expr` 未定義で `no method named lower_match_expr` として失敗。
- GREEN: seam test が scrutinee の `I64Const`、`LocalSet`、arm body の順序を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（156 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは `Expr::Match` production の責務分離だけを扱う。pattern lowering 全体の意味論
parity、native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
