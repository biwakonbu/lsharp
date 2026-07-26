# ADR: tooling metadata test suite の責務分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-tooling/src/metadata_test_tests.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), [legacy tooling metadata test split](decisions-legacy-tooling-metadata-test-split.md)

## Context

`metadata_test_tests.rs` は metadata がない source、parse/I/O diagnostics、legacy invariant、
canonical assertion/case、deterministic property smoke の 36 tests を 742 行の単一 file に保持していた。
fixture helper と各 contract profile の責務が混在し、metadata test の仕様回帰を profile 単位で
review しにくかった。

## Decision

- shared `unique_temp_dir` と既存の `metadata_test::tests` facade は
  `metadata_test_tests.rs` に残す。
- 基本 source/diagnostic/invariant tests を `metadata_test_tests/basic.rs`、canonical
  assertion/case tests を `metadata_test_tests/canonical.rs`、property profile tests を
  `metadata_test_tests/property.rs` へ移動する。
- `include!` で三つの test fragment を同一 namespace に取り込み、既存の
  `metadata_test::tests::test_run_metadata_tests_*` test path、fixture、test body を維持する。

## Evidence

- RED: facade の include/module 宣言直後に child file 不在の `E0583` を確認。
- GREEN: `cargo test -p lsharp-tooling metadata_test::tests::test_run_metadata_tests_ -- --nocapture` — 36 passed。
- Package: `RUST_MIN_STACK=33554432 cargo test -p lsharp-tooling -- --nocapture` — unit 134、
  doc-test 0 が全て pass。
- `cargo clippy -p lsharp-tooling --all-targets -- -D warnings`、`cargo check --workspace`、
  対象 Rust 2024 `rustfmt --check`、`git diff --check`、`bash scripts/audit_docs.sh` が pass。
- parent は 742 行から 24 行へ縮小し、`basic.rs` 181 行、`canonical.rs` 117 行、
  `property.rs` 421 行となった。

## Consequences

metadata test の profile ごとの変更範囲と回帰実行を独立に追跡できる。production API、runtime、
fixture semantics、既存 test namespace は変更しない。metadata runner の production 責務分割、
selfhost/native parity、I-01 / I-08 aggregate は未完了である。
