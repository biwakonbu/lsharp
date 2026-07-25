# ADR: metadata checker legacy invariant production split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/metadata_check.rs`
- Related: `I-01`, `I-08`, `docs/adr/decisions-legacy-metadata-check-test-generation-split.md`

## Context

`metadata_check.rs` は metadata diagnostics、legacy `:invariant` の synthetic probe、
reference helper、executable test generation を一つの production module に抱えていた。
reference helper と test generation は既に分離済みだが、legacy invariant の型検査が親に
残り、legacy contract の変更と通常の metadata diagnostics の review 境界が混ざっていた。

## Decision

- `LegacyInvariantProbe` と `check_legacy_invariant_types` を
  `metadata_check/legacy.rs`（131 行）に移す。
- `check_metadata` は `legacy::check_legacy_invariant_types` を呼ぶだけとし、診断型と
  既存の metadata API は親 module に残す。
- synthetic probe の生成、unknown reference の fail-closed 条件、Infer の結果から
  `Bool` を検査する順序、診断 message/span/function name は変更しない。
- module seam test で legacy probe の型診断を直接固定し、package regression で既存の
  metadata contract/property/source validation parity を確認する。

## Evidence

- RED: `legacy` module 未作成時は `legacy_invariant_module_exposes_type_probe` が
  `file not found for module legacy` で失敗。
- GREEN: `legacy_invariant_module_exposes_type_probe` が `:invariant` の非 Bool 診断を確認。
- `cargo test -p lsharp-types -- --nocapture`（211 unit tests、全 integration/doc tests）
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`
- `cargo check --workspace --message-format=short`
- Rust 2024 rustfmt、`git diff --check`

## Boundary

これは legacy `:invariant` probe の production 責務分離だけを扱う。通常の metadata
diagnostics、`check_defn_metadata`、legacy migration、parser/lexer production split、
native/selfhost parity、I-01 / I-08 aggregate の完了を意味しない。
