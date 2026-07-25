# ADR: `lsharp-types/types.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/types.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`types.rs` は `Type`、`TypeScheme`、`Substitution`、`TypeEnv` と制約 schema を実装し、同じ
ファイル末尾に置換の空 map、scheme binding、長い型変数連鎖、cycle safety を確認する 4 件の
回帰テストを保持していた。test-only fixture を分離すると、型 schema/置換 production と
regression fixture の ownership/review 境界を明確にできる。

## Decision

- `Type` / `TypeScheme` / `Substitution` / `TypeEnv` / constraint schema の semantics と公開
  API は変更しない。
- `#[cfg(test)] mod apply_subst_tests` の 4 件を
  `crates/lsharp-types/src/types_tests.rs` へ移動する。
- `types.rs` は `#[cfg(test)] #[path = "types_tests.rs"] mod apply_subst_tests;` で既存の
  `types::apply_subst_tests` namespace を維持する。
- substitution empty/bound/chain/cycle contract は同一コミットで変更しない。

## Evidence

- 分離前後の `types::apply_subst_tests` focused gate: 4 passed。
- `types.rs` は 527 行から 479 行へ、`types_tests.rs` は 48 行となった。
- `cargo test -p lsharp-types`: unit 209 passed、integration 49 passed、doc-tests 0 passed / 0 failed。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、Rust 2024 rustfmt、
  `git diff --check`、`bash scripts/audit_docs.sh` は pass。

## Consequences

型 schema/置換 production と回帰 fixture の ownership/review 境界が明確になり、4 件のテストを
単独で再実行できる。他の大規模 types file、production の責務分割、I-01 / I-08 aggregate は
未完了であるため、TODO の partial slice を維持する。
