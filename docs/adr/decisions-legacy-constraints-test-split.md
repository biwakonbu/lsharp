# ADR: `constraints.rs` の inline test module 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/constraints.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

制約評価・階層解決・型変換・runtime check の production 実装と、4 つの inline test module（合計 43 件）が同じ file に混在していた。制約実装の差分レビューと failure layer の切り分けに test body が混ざり、今後の production 分割の前提も作りにくかった。

## Decision

- 既存の `#[cfg(test)]` module 境界と module 名を維持し、`constraints/{tests,hierarchy_tests,conversion_tests,runtime_check_tests}.rs` へ test body だけを移動する。
- `constraints.rs` には module 宣言だけを残し、production API、内部関数、制約評価の semantics は変更しない。
- `constraints::tests::*` など既存の test namespace と fixture は維持する。

## Evidence

- 分離前後の focused gate `cargo test -p lsharp-types constraints:: -- --nocapture`: 43 passed。
- `constraints.rs` は 1,089 行から 543 行へ縮小し、4 test file は 113〜177 行に収まった。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check`、`bash scripts/audit_docs.sh` が pass。

## Consequences

制約 test を production 差分から独立してレビュー・実行できるようになり、今後の `constraints.rs` production 分割の衝突面を減らせる。制約 production の責務分割、`I-01` / `I-08` aggregate、native/selfhost parity は後続であり、この ADR の verified slice だけで完了扱いにはしない。
