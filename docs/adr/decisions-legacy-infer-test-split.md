# ADR: `lsharp-types/infer.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/infer.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`infer.rs` は Hindley–Milner 型推論、制約登録、import/module visibility、GADT/kind/computation expression
処理に加えて、11 個の test module と proptest fixture を同じファイル末尾に保持していた。test-only 部分を
分離すると、型推論 production と回帰/property fixture の ownership と review 境界を明確にできる。

## Decision

- `Infer`、`TypeError`、unify/generalize、module/import resolution の production API と semantics は変更しない。
- `#[cfg(test)]` の 11 test module（92 tests）を `crates/lsharp-types/src/infer_tests.rs` へ移動する。
- 親では `#[cfg(test)] include!("infer_tests.rs");` を使い、既存の `infer::tests`、`private_tests`、
  `nested_module_infer_tests` などの module path と private item access を維持する。
- test body、property generator、fixture、assertion は変更しない。infer production の責務分割は後続とする。

## Evidence

- 分離前後の `cargo test -p lsharp-types infer -- --nocapture`: 92 passed。
- `infer.rs` は 4055 行から 2789 行へ、`infer_tests.rs` は 1268 行となった。
- `cargo test -p lsharp-types`: unit 209、integration 49、doc-tests 0 の合計 258 passed。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings` は pass。
- 対象2ファイルの Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

型推論 production と 92 件の回帰/property test fixture の ownership/review 境界が明確になり、既存 module
path を保ったまま focused gate を単独実行できる。infer の production 責務分割、他の大規模 Rust file 分割、
I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
