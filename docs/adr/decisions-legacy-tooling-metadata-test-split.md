# ADR: `lsharp-tooling/metadata_test.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-tooling/src/metadata_test.rs`, `crates/lsharp-tooling/src/metadata_test_tests.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`metadata_test.rs` は metadata contract の読み込み・診断・test program 生成と Wasm 実行を
担当する production code に、35 件の metadata / property / canonical case 回帰テストを
同居させていた。test-only fixture を分離し、production と検証 fixture の ownership/review
境界を明確にする。

## Decision

- `MetadataTestRun`、`test_kind_label`、`run_metadata_tests` の公開 API と実行 semantics は変更しない。
- `#[cfg(test)] mod tests` を `crates/lsharp-tooling/src/metadata_test_tests.rs` へ移動する。
- `metadata_test.rs` は `#[cfg(test)] #[path = "metadata_test_tests.rs"] mod tests;` を通じて既存の
  `metadata_test::tests` namespace を維持する。
- metadata property の既知の `LS2005` vacuous failure boundary や他の CLI surface は同じコミットで変更しない。

## Evidence

- 分離前後の `metadata_test` focused gate は同じ 35 件を収集し、33 passed / 2 failed。
- 失敗は既知の `LS2005` vacuous boundary（`test_run_metadata_tests_executes_bool_property_binder`、
  `test_run_metadata_tests_rejects_bool_property_above_two_cases`）で、分離差分とは無関係である。
- `metadata_test.rs` は 855 行から 133 行へ、`metadata_test_tests.rs` は 722 行となった。

## Consequences

metadata test runner の production code と test fixture の変更境界が明確になり、既存 namespace
を壊さず focused gate を実行できる。metadata property failure の修正、追加の production 分割、
`I-01` / `I-08` aggregate は未完了のため、TODO の partial slice は維持する。
