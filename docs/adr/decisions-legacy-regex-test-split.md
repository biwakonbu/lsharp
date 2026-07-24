# ADR: `regex/mod.rs` の inline test module 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/regex/mod.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

NFA regex matcher の production 実装と 25 件の regex behavior test が同じ 1,277 行の module に混在していた。既存の DFA tests と production parser/matcher の差分を独立してレビューし、pattern feature regression の切り分けを容易にする必要があった。

## Decision

- 既存の `#[cfg(test)] mod tests` 境界を維持し、test body を `regex/tests.rs` へ移動する。
- `regex/mod.rs` には test module 宣言だけを残し、`parse_regex`、matcher、capture、shared pattern API の semantics は変更しない。
- `regex::tests::*` の namespace と既存 fixture は維持する。

## Evidence

- 分離前後の focused gate `cargo test -p lsharp-types regex:: -- --nocapture`: 38 passed（regex 25、DFA 13）。
- `regex/mod.rs` は 1,277 行から 1,076 行へ縮小し、`regex/tests.rs` は 200 行となった。
- `cargo test -p lsharp-types`、`cargo clippy -p lsharp-types --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check`、`bash scripts/audit_docs.sh` が pass。

## Consequences

regex behavior tests を matcher production の差分から独立してレビュー・実行できるようになった。regex production の parser/matcher 分割、`I-01` / `I-08` aggregate、native/selfhost parity は後続であり、この ADR の verified slice だけで完了扱いにはしない。
