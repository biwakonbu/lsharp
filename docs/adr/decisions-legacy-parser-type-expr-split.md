# ADR: parser type-expression production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-parser-test-split.md`

## Context

`parser.rs` は declaration、expression、pattern、metadata、evidence と type-expression
parsing を一つの production module に抱えていた。parser の inline tests は既に分離済みで、
型式 parser は独立した再帰・stack-growth boundary として切り出せるため、selfhost Parser
の共有差分に触れず Rust parser の review 単位を狭める。

## Decision

- `parse_type_expr` と `parse_type_expr_inner` を `parser/type_expr.rs`（91 行）へ移す。
- child module の `impl Parser` と `pub(super)` entrypoint を使い、既存の `self.parse_type_expr()`
  呼び出しと `Parser` 公開 API は維持する。
- named/variable、record、function、type application の AST projection、recursive
  `stacker::maybe_grow` boundary、診断 message/span は変更しない。
- module seam test で function type projection を直接固定し、syntax package 回帰で parser
  behavior parity を確認する。

## Evidence

- RED: `type_expr` module 未作成時は `type_expr_module_exposes_type_parser` が
  `file not found for module type_expr` で失敗。
- GREEN: seam test が `(-> Int a)` を `TypeExpr::Fun` と `TypeExpr::Var` に投影。
- `cargo test -p lsharp-syntax -- --nocapture`（165 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`

## Boundary

これは type-expression parser の production 責務分離だけを扱う。parser の expression/decl
分割、lexer error/API、selfhost/native parity、I-01 / I-08 aggregate の完了を意味しない。
