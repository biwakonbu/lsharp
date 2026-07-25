# ADR: parser pattern production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-parser-expr-split.md`

## Context

`parser.rs` は declaration、expression、pattern、metadata、evidence の production を
一つの module に抱えていた。expression production の分離に続き、pattern parser を
独立した再帰・stack-growth boundary として切り出すことで、selfhost Parser の共有差分に
触れず Rust parser の変更単位を狭める。

## Decision

- `parse_pattern` と `parse_pattern_inner` を `parser/pattern.rs`（101 行）へ移す。
- `parse_pattern` は `pub(super)` の module 内 entrypoint とし、expression child module と
  既存の親 parser 呼び出しから利用できるようにする。公開 `Parser` API は拡張しない。
- wildcard/variable/constructor/literal/record pattern の AST projection、token
  consumption、recursive `stacker::maybe_grow` boundary、診断 message/span は変更しない。
- module seam test で constructor pattern projection を直接固定し、syntax package 回帰で
  parser behavior parity を確認する。

## Evidence

- RED: `pattern` module 未作成時は `pattern_module_exposes_pattern_parser` が
  `file not found for module pattern` で失敗。
- GREEN: seam test が `(Some x)` を一引数の `Pattern::Constructor` に投影。
- `cargo test -p lsharp-syntax -- --nocapture`（167 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`、docs audit

## Boundary

これは pattern parser の production 責務分離だけを扱う。parser の declaration/metadata/
evidence 分割、lexer error/API、selfhost/native parity、I-01 / I-08 aggregate の完了を意味しない。
