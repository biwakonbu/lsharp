# ADR: regex matcher の capture/backreference 責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/regex/matcher.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-regex-production-split.md`

## Context

`regex/matcher.rs` は通常の NFA backtracking、capture/backreference、bounded repeat、
先読み判定、DFA/NFA の入口を同じファイルで保持していた。capture 経路は通常 matcher の
内部 helper に依存する一方、変更時に相互の責務境界をレビューしにくく、production file の
追加分割を妨げていた。

## Decision

- capture/backreference 用の `regex_match_with_captures` と capture-aware bounded repeat を
  `regex/matcher_advanced.rs` へ移動する。
- 通常 NFA matcher の `regex_match`、複雑ノードの repeat、group span helper は
  `pub(super)` の内部 seam として維持し、capture module から再利用する。
- `simple_pattern_match`、`has_advanced_features`、`regex::*` の crate-private API と
  backreference / lookahead / bounded-repeat の判定順序・step limit は変更しない。
- capture module の seam test を `regex::tests` に置き、既存 namespace と公開導線を保つ。

## Evidence

- RED: `matcher_advanced` module 未作成で focused seam test が `E0583` となることを確認。
- GREEN: `cargo test -p lsharp-types regex --lib -- --nocapture` — 40 passed。
- Package gate: `RUST_MIN_STACK=33554432 cargo test -p lsharp-types -- --nocapture` —
  unit 213、全 integration targets、doc-test 0 が全て pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、変更対象 Rust 2024 rustfmt、
  `git diff --check` が pass。
- `matcher.rs` は 699 行から 443 行へ、`matcher_advanced.rs` は 270 行となり、
  advanced capture responsibility が独立した。

## Consequences

capture/backreference の実装を通常 NFA matcher から独立してレビュー・検証できる。
既存の判定結果、crate-private API、DFA fast path は維持する。regex parser の追加分割、
matcher algorithm の改善、selfhost/native parity、I-01 / I-08 aggregate は未完了である。
