# ADR: parser metadata directive production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-parser-pattern-split.md`

## Context

`parser.rs` は declaration、expression、pattern、metadata、evidence の production を
一つの module に抱えていた。expression/pattern production の分離に続き、metadata
directive と property parser を独立した review 単位にすることで、contract metadata の
変更境界を Rust parser の他の production から切り離す。

## Decision

- `try_parse_metadata` と legacy/intent/evidence-form projection、case/assert/property
  helper を `parser/metadata.rs`（484 行）へ移す。
- module 内 entrypoint は `pub(super)` とし、既存の `parse_defn` 呼び出しと公開 `Parser`
  API は維持する。evidence record field parser は別の shared boundary として親に残し、
  metadata child からの呼び出しだけ `pub(super)` で接続する。
- metadata AST projection、source order/form spans、token consumption、diagnostic
  message/span は変更しない。
- module seam test で `:doc` projection を直接固定し、syntax package 回帰で parser
  behavior parity を確認する。

## Evidence

- RED: `metadata` module 未作成時は `metadata_module_exposes_directive_parser` が
  `file not found for module metadata` で失敗。
- GREEN: seam test が `:doc "hello"` を `Metadata.doc` へ投影。
- `cargo test -p lsharp-syntax -- --nocapture`（168 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`、docs audit

## Boundary

これは metadata directive/property parser の production 責務分離だけを扱う。evidence
field parser、parser の declaration production、lexer error/API、selfhost/native parity、
I-01 / I-08 aggregate の完了を意味しない。
