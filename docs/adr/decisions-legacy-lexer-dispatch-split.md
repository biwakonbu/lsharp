# ADR: lexer token dispatch production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/lexer.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-lexer-tokenization-split.md`

## Context

`lexer.rs` は stable diagnostic API、token stream orchestration、UTF-8 cursor helper、
delimiter/operator dispatch、文字列・数値・symbol tokenization を一つの module に抱えて
いた。tokenization の分離に続き、単一文字 token と複合 operator の dispatch を独立した
review 単位にすることで、lexical surface の変更境界を cursor と tokenization helper から
切り離す。

## Decision

- `next_token` と delimiter/operator dispatch、tokenization helper の呼び分け、unexpected
  character boundary を `lexer/dispatch.rs`（98 行）へ移す。
- `next_token` は `tokenize` から利用する `pub(super)` seam とし、公開 `Lexer` API、
  `LexError` code/span、token kind、source span、UTF-8 cursor semantics は変更しない。
- `dispatch` module seam test で `|>`、`->`、`~@` と delimiter の token projection を直接
  固定し、syntax package 回帰で既存 lexer behavior parity を確認する。

## Evidence

- RED: `dispatch` module 未作成時は `dispatch_module_exposes_delimiter_and_operator_scanner`
  が `file not found for module dispatch` で失敗。
- GREEN: seam test が delimiter、pipeline symbol、arrow、splice-unquote を期待 token へ投影。
- `cargo test -p lsharp-syntax -- --nocapture`（171 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`

## Boundary

これは lexer token dispatch の production 責務分離だけを扱う。lexer error/API の設計拡張、
selfhost/native parity、I-01 / I-08 aggregate の完了を意味しない。
