# ADR: parser evidence record production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-parser-metadata-split.md`

## Context

`parser.rs` は declaration、expression、pattern、metadata、evidence の production を
一つの module に抱えていた。expression/pattern/metadata production の分離に続き、
required evidence fields と optional sampling fields の parser を独立した review 単位に
することで、evidence contract の変更境界を他の parser production から切り離す。

## Decision

- `parse_evidence_form` と evidence field decoding、required-field、sampling helper を
  `parser/evidence.rs`（313 行）へ移す。
- evidence entrypoint は `pub(super)` とし、metadata child からの既存呼び出しを維持する。
  共有する metadata string parser は親の `pub(super)` seam に残す。
- `EvidenceForm` の AST projection、required field semantics、duplicate/unknown field の
  diagnostics、source span、token consumption、optional `shrinks` / `coverage` の既定値を
  変更しない。
- module seam test で required fields の record projection を直接固定し、syntax package
  回帰で parser behavior parity を確認する。

## Evidence

- RED: `evidence` module 未作成時は `evidence_module_exposes_record_parser` が
  `file not found for module evidence` で失敗。
- GREEN: seam test が required fields を `EvidenceForm` へ投影し、stable ID と subject を保持。
- `cargo test -p lsharp-syntax -- --nocapture`（169 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`、docs audit

## Boundary

これは evidence record field parser の production 責務分離だけを扱う。parser の
declaration production、lexer error/API、selfhost/native parity、I-01 / I-08 aggregate の
完了を意味しない。
