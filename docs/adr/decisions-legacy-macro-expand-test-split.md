# ADR: `macro_expand.rs` の inline test module 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-syntax/src/macro_expand.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

マクロ展開器本体と、通常マクロ・P10-5 built-in macro・computation macro の 3 つの inline test module（合計 35 件）が同じ file に混在していた。展開ロジックの差分レビューと test failure の切り分けに test body が混ざり、macro feature ごとの検証を独立して扱いにくかった。

## Decision

- 既存の `#[cfg(test)]` module 境界と module 名を維持し、`macro_expand/{tests,tests_p10_5,tests_computation_macro}.rs` へ test body だけを移動する。
- `macro_expand.rs` には module 宣言だけを残し、公開 API、展開順序、再帰制限、trace semantics は変更しない。
- `macro_expand::tests::*` など既存の test namespace と fixture は維持する。

## Evidence

- 分離前後の focused gate `cargo test -p lsharp-syntax macro_expand:: -- --nocapture`: 35 passed。
- `macro_expand.rs` は 1,697 行から 995 行へ縮小し、3 test file は 157〜368 行に収まった。
- `cargo test -p lsharp-syntax`、`cargo clippy -p lsharp-syntax --all-targets -- -D warnings`、changed files の rustfmt check、`git diff --check`、`bash scripts/audit_docs.sh` が pass。

## Consequences

macro feature ごとの test を production 差分から独立してレビュー・実行できるようになった。macro production の本体・built-in 分割、`I-01` / `I-08` aggregate、selfhost/native parity は後続であり、この ADR の verified slice だけで完了扱いにはしない。
