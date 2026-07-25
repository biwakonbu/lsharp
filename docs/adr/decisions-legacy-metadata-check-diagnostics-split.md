# ADR: metadata checker diagnostics production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/metadata_check.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-metadata-check-legacy-split.md`

## Context

`metadata_check.rs` は、全体 orchestration に加えて関数 metadata の `:params`、
`:see-also`、`:doc`、`:invariant`、`:example` の構造診断を同じ production module に
抱えていた。reference helper、test generation、legacy `:invariant` type probe は既に
分離済みであり、通常の metadata diagnostics も独立した review/変更単位にする必要があった。

## Decision

- `check_defn_metadata`、`check_invariant`、`check_example` を
  `metadata_check/diagnostics.rs`（156 行）へ移す。
- `check_metadata` は diagnostics module の `pub(super)` entrypoint を呼び、診断型
  `Severity` / `MetadataDiagnostic` と公開 `check_metadata` API は親 module に残す。
- `:params` の error/warning、`:see-also`・`:doc` の参照確認、`:invariant` / `:example`
  の scope 判定、診断の順序・message・span は変更しない。
- seam test で unknown parameter の error/warning projection を直接固定し、package
  regression で既存 metadata contract/property/source validation parity を確認する。

## Evidence

- RED: `diagnostics` module 未作成時は `diagnostics_module_exposes_defn_metadata_check` が
  `file not found for module diagnostics` で失敗。
- GREEN: 同 seam test が unknown parameter の error と omitted parameter の warning を確認。
- `cargo test -p lsharp-types -- --nocapture`（212 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`、`bash scripts/audit_docs.sh`

## Boundary

これは metadata structure diagnostics の production 責務分離だけを扱う。`Severity` /
`MetadataDiagnostic` 型の再設計、legacy metadata migration、parser/lexer production split、
native/selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
