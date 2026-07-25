# ADR: `lsharp-tooling/api_doc.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-tooling/src/api_doc.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`api_doc.rs` は AST/type result から API document を構築し、file/package discovery と
diagnostic code を保持する production code と、metadata/signature、module ordering、stdlib
public API、parse error、file I/O error を確認する 7 件の回帰テストを同じファイルに保持していた。
test-only fixture を分離すると、API document builder production と metadata/file fixture の
ownership/review 境界を明確にできる。

## Decision

- `ApiDoc` / `ApiModule` / `ApiFunction` schema、program/type lowering、file/package discovery、
  signature rendering、module doc extraction と diagnostic semantics は変更しない。
- `#[cfg(test)] mod tests` の 7 件を
  `crates/lsharp-tooling/src/api_doc_tests.rs` へ移動する。
- `api_doc.rs` は `#[cfg(test)] #[path = "api_doc_tests.rs"] mod tests;` で既存の
  `api_doc::tests` namespace を維持する。
- API doc JSON shape、stdlib public metadata contract、parse/I/O error code は同一コミットで
  変更しない。

## Evidence

- 分離前後の `api_doc::tests` focused gate: 7 passed。
- `api_doc.rs` は 533 行から 326 行へ、`api_doc_tests.rs` は 207 行となった。
- `cargo test -p lsharp-tooling --doc`: 0 passed / 0 failed。
- `cargo test -p lsharp-tooling`: 130 passed / 2 failed。失敗は既存の metadata property
  LS2005 vacuity boundary であり、`metadata_test::tests::test_run_metadata_tests_executes_bool_property_binder`
  は deterministic Bool prefix、`metadata_test::tests::test_run_metadata_tests_rejects_bool_property_above_two_cases`
  は LS3002 を期待するが、いずれも現状は LS2005 を返す。今回の `api_doc` 分離は metadata_test と
  その production path を変更していない。
- `cargo clippy -p lsharp-tooling --all-targets -- -D warnings`、Rust 2024 rustfmt、
  `git diff --check`、`bash scripts/audit_docs.sh` は pass。

## Consequences

API document builder production と metadata/file fixture の ownership/review 境界が明確になり、
7 件の回帰テストを単独で再実行できる。tooling の metadata property failure boundary、他の
大規模 Rust file、production の追加責務分割、I-01 / I-08 aggregate は未完了であるため、TODO の
partial slice を維持する。
