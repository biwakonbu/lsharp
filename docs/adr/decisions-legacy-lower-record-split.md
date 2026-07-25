# ADR: lower record production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lower/expr.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lower-lambda-split.md`

## Context

`lower/expr.rs` の `Expr::RecordLit`、`Expr::FieldAccess`、`Expr::RecordUpdate` production は、
GC record の field ordering、`StructGet` / `StructNew`、型名解決と fallback を巨大な
`lower_expr` match に抱えていた。record の layout と更新 semantics が expression production
本体に混在し、変更衝突と review 単位を増やしていた。

## Decision

- `RecordLit`、`FieldAccess`、`RecordUpdate` の lowering を
  `lower/expr/record.rs`（183 行）の `pub(super)` helper へ移動する。
- 親 `expr.rs` は AST の必要な引数を record module helper へ渡すだけにし、field order、
  `StructGet` / `StructNew`、型名解決、fallback、error/span、backend semantics を維持する。
- record module seam test で、source field order と定義 field order が異なる GC record literal
  でも定義順に値を積む contract を固定する。

## Evidence

- RED: `record` module 未作成時は
  `record_module_preserves_field_order_for_gc_structs` が `file not found for module record`
  と `lower_record_lit` 未定義で失敗。
- GREEN: seam test が GC record literal の値を定義 field order で `StructNew` へ積むことを確認。
- `cargo test -p lsharp-ir lower:: -- --nocapture`（155 passed）
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- 変更対象の Rust 2024 rustfmt check、`git diff --check`
- `bash scripts/audit_docs.sh`（エラー 0、警告 0）

## Boundary

これは record production の責務分離だけを扱う。`lower_expr` 全 production の意味論 parity、
native/runtime artifact、selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
`lsharp-ir` package 全体には既知の selfhost fixture failure
（`test_compile_multi_file_incremental_clean_formatter_trio_cache_hit_succeeds` における
`vector-push-pair-rooted-v3` 未定義）が残っており、今回の差分外として扱う。
