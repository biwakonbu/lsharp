# ADR: `lexer.rs` の inline test module 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-syntax/src/lexer.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`lexer.rs` は字句解析 production と通常 24 件、UTF-8 9 件、UTF-8 comment 4 件の計 37 件の test が同じ約 890 行の file に混在していた。production の変更と fixture のレビュー境界を分け、lexer production split の前提を作る必要があった。

## Decision

- 既存の `#[cfg(test)] mod tests`、`utf8_tests`、`utf8_comment_tests` namespace を維持する。
- test body を `lexer/tests.rs`、`lexer/utf8_tests.rs`、`lexer/utf8_comment_tests.rs` へ移動する。
- `lexer.rs` は production と module declarations だけを持ち、tokenization semantics、公開 `Lexer` / `LexError` API、fixture は変更しない。

## Evidence

- 分離前後の focused `cargo test -p lsharp-syntax lexer:: -- --nocapture`: 37 passed。
- `cargo test -p lsharp-syntax`: unit 163、integration 2/9/1 が全て pass。
- `lexer.rs` は 890 行から 378 行へ縮小し、tests files は 335/134/38 行となった。
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check` が pass。

## Consequences

lexer behavior tests を production lexer の差分から独立してレビュー・実行できるようになった。lexer production の責務分割、syntax property 拡張、`I-01` / `I-08` aggregate、selfhost/native parity は後続であり、この verified slice だけで完了扱いにはしない。
