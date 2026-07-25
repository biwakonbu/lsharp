# ADR: lexer tokenization production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/lexer.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lexer-test-split.md`

## Context

`lexer.rs` は `Lexer` の状態管理と、whitespace/comment skip、文字列・数値・symbol の
tokenization を一つの production module に抱えていた。lexer の inline tests は既に分離済み
だが、tokenization の変更境界が状態管理・dispatch と混ざっており、parser と並行した
frontend review の単位を狭めていた。

## Decision

- `skip_whitespace_and_comments`、`lex_string`、`lex_number`、`lex_symbol` を
  `lexer/tokenization.rs`（182 行）へ移す。
- 親 `Lexer` は source state、tokenize loop、delimiter/quote dispatch、symbol boundary helper
  を保持し、tokenization module の `pub(super)` functions を呼び出す。
- `LexError`、`Lexer` の公開 API、token kind、UTF-8/escape/number parsing、診断 span は変更しない。
- module seam test で number/symbol scanner の token projection を直接固定し、syntax package
  回帰で既存 lexer/parser behavior parity を確認する。

## Evidence

- RED: `tokenization` module 未作成時は `tokenization_module_exposes_number_and_symbol_scanners`
  が `file not found for module tokenization` で失敗。
- GREEN: seam test が `41` と `defn` の token kind を確認。
- `cargo test -p lsharp-syntax -- --nocapture`（164 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`

## Boundary

これは lexer tokenization production の責務分離だけを扱う。`LexError` の診断体系変更、
parser production split、selfhost/native parity、I-01 / I-08 aggregate の完了を意味しない。
