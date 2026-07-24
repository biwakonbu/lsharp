# ADR: `parser.rs` の inline test module 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-syntax/src/parser.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

Rust parser の production 実装と 61 件の parser / transition / computation test が同じ 2,524 行の file に混在していた。parser の実装変更と metadata/computation regression のレビューを分離できず、failure layer の切り分けにも余分なコストがあった。

## Decision

- `#[cfg(test)]` の既存 module 境界を維持し、`parser/tests.rs`、`parser/transitions_tests.rs`、`parser/computation_tests.rs` へ test body だけを移動する。
- `parser.rs` からは `mod` 宣言だけを行い、production parser の公開 API と parse semantics は変更しない。
- test module の名前空間 (`parser::tests::*` など) と fixture は維持する。

## Evidence

- 移動前後の parser focused gate: 61 passed。
- `cargo clippy -p lsharp-syntax --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check` が pass。
- `parser.rs` は 2,524 行から 1,790 行へ縮小し、test body を専用 files へ分離した。

## Consequences

parser tests を production parser の差分から独立してレビュー・実行できるようになった。parser production の expr/decl 分割、`I-01` / `I-08` aggregate、native/selfhost parity は後続であり、この ADR の verified slice だけで完了扱いにはしない。
