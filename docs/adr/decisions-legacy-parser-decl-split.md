# ADR: parser declaration production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-parser-evidence-split.md`

## Context

`parser.rs` は declaration、expression、pattern、metadata、evidence の production を
一つの module に抱えていた。expression/pattern/metadata/evidence production の分離に
続き、トップレベル declaration とその補助 parser を独立した review 単位にすることで、
宣言形式の変更境界を式・型式・metadata の parser から切り離す。

## Decision

- `parse_decl` と `defn`、type/record/type-alias/constraint、module/import、trait/impl、
  private、macro、computation-builder、where/variant/parameter helper を
  `parser/decl.rs`（701 行）へ移す。
- `parse_decl` は親の recovery loop から呼べる `pub(super)` seam とし、`parse_params` は
  sibling の expression parser が既存の function parameter path を利用できる
  `pub(super)` seam とする。公開 `Parser` API は変更しない。
- AST projection、declaration spans、token consumption、diagnostic message/span、
  nested module/private/impl parsing を変更しない。
- module seam test で top-level `defn` declaration の projection を直接固定し、syntax
  package 回帰で既存 declaration behavior parity を確認する。

## Evidence

- RED: `decl` module 未作成時は `declaration_module_exposes_top_level_parser` が
  `file not found for module decl` で失敗。
- GREEN: seam test が `(defn identity [x] x)` を `Decl::Defn` へ投影。
- `cargo test -p lsharp-syntax -- --nocapture`（170 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`

## Boundary

これは Rust parser の declaration production 責務分離だけを扱う。lexer error/API、
selfhost/native parity、parser 全体の I-01 / I-08 aggregate、公開 command の Rust-free
完了を意味しない。
