# ADR: parser expression production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-parser-type-expr-split.md`

## Context

`parser.rs` は declaration、expression、pattern、metadata、evidence の production を
一つの module に抱えていた。type-expression production の分離に続き、expression parser を
独立した再帰・stack-growth boundary として切り出すことで、selfhost Parser の共有差分に
触れず Rust parser の変更単位を狭める。

## Decision

- `parse_expr`、`parse_expr_inner` と literal/list/brace、control-flow、computation、
  annotation、application の helper を `parser/expr.rs`（381 行）へ移す。
- child module の `impl Parser` を使い、既存の `Parser::parse_expr` 公開 API と親 module
  からの呼び出しを維持する。
- AST projection、token consumption、recursive `stacker::maybe_grow` boundary、診断
  message/span は変更しない。
- module seam test で `if` expression projection を直接固定し、syntax package 回帰で
  parser behavior parity を確認する。

## Evidence

- RED: `expr` module 未作成時は `expr_module_exposes_expression_parser` が
  `file not found for module expr` で失敗。
- GREEN: seam test が `(if true 1 2)` を `Expr::If` に投影。
- `cargo test -p lsharp-syntax -- --nocapture`（全 unit / integration / doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`、docs audit

## Boundary

これは expression parser の production 責務分離だけを扱う。parser の declaration/pattern/
metadata/evidence 分割、lexer error/API、selfhost/native parity、I-01 / I-08 aggregate の
完了を意味しない。
