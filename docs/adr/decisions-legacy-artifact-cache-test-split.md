# ADR: `lsharp-tooling/artifact_cache.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-tooling/src/artifact_cache.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`artifact_cache.rs` は compile cache envelope の key/fingerprint 検証、atomic 保存、deterministic
entry/byte trim を実装し、同じファイル末尾に cache round-trip、破損 envelope、trim、I/O error
code を確認する 6 件の回帰テストと fixture helper を保持していた。test-only fixture を分離すると、
cache production と filesystem/cache regression fixture の ownership/review 境界を明確にできる。

## Decision

- `ArtifactCache`、envelope schema、fingerprint validation、trim policy、公開 API と production
  semantics は変更しない。
- `#[cfg(test)] mod tests` の 6 件と helper を
  `crates/lsharp-tooling/src/artifact_cache_tests.rs` へ移動する。
- `artifact_cache.rs` は `#[cfg(test)] #[path = "artifact_cache_tests.rs"] mod tests;` で既存の
  `artifact_cache::tests` namespace を維持する。
- cache payload、deterministic deletion、caller-owned metadata、LS5001 I/O error の fixture と
  assertion は変更しない。metadata property の既知 failure は別タスクとして残す。

## Evidence

- 分離前後の `artifact_cache::tests` focused gate: 6 passed。
- `artifact_cache.rs` は 468 行から 222 行へ、`artifact_cache_tests.rs` は 246 行となった。
- `cargo test -p lsharp-tooling`: 130 passed / 2 failed。失敗は既存
  `metadata_test::tests::test_run_metadata_tests_executes_bool_property_binder` と
  `metadata_test::tests::test_run_metadata_tests_rejects_bool_property_above_two_cases` の
  LS2005 vacuity boundary であり、今回の test-only 移動とは無関係。
- `cargo clippy -p lsharp-tooling --all-targets -- -D warnings`、
  `cargo test -p lsharp-tooling --doc` (0/0)、Rust 2024 rustfmt、`git diff --check`、
  `bash scripts/audit_docs.sh` は pass。

## Consequences

artifact cache production と filesystem regression fixture の ownership/review 境界が明確になり、
6 件の回帰テストを単独で再実行できる。metadata property LS2005 boundary、他の tooling production
split、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
