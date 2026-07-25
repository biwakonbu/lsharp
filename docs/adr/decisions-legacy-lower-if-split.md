# ADR: lower if production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-do-split.md`

## Context

`lower/expr.rs` の `Expr::If` production は、条件式の lowering、Bool の i64→i32 変換、
Wasm の if/else/end 命令、then/else branch の lowering を `lower_expr` の match arm に
抱えていた。control-flow の命令列と他の expression production が混在し、変更衝突と review
単位を増やしていた。

## Decision

- `Expr::If` の lowering を `lower/expr/if_expr.rs`（26 行）の `lower_if` へ移動する。
- 親 `expr.rs` は condition と二つの branch を helper へ渡すだけにし、評価順序、Bool の変換、
  `If(I64)` / `Else` / `End` の命令列、error semantics を維持する。
- if module seam test で、condition → conversion → then → else の順序を固定する。

## Evidence

- RED: seam test は `lower_if` 未定義で `no method named lower_if` として失敗。
- GREEN: seam test が condition、branch、control-flow 命令の順序を確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（160 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは if production の責務分離だけを扱う。全 control-flow/backend/native/runtime parity、
selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
