# ADR: `knowledge.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-docs/src/knowledge.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`knowledge.rs` は Knowledge JSON の production schema/serialization API と、knowledge root・TypeKind variant・constrained type の 3 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、knowledge schema の変更と JSON fixture の ownership/review 境界を明確にできる。

## Decision

- `Knowledge`、各情報構造体、`TypeKind`/`DependencyKind`、`to_json` の公開契約と semantics は変更しない。
- `#[cfg(test)] mod tests` の 3 件を `crates/lsharp-docs/src/knowledge_tests.rs` へ移動する。
- `knowledge.rs` は `#[cfg(test)] #[path = "knowledge_tests.rs"] mod tests;` で既存の `knowledge::tests` namespace を維持する。
- Knowledge JSON schema、serde attributes、docs generation 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `knowledge::tests` focused gate: 3 passed。
- `cargo test -p lsharp-docs`: 23 passed、doc-tests 0 passed。
- `cargo clippy -p lsharp-docs --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `knowledge.rs` は 207 行から 120 行へ、`knowledge_tests.rs` は 87 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

knowledge production schema と JSON fixture の ownership/review 境界が明確になり、3 件の回帰テストを単独で再実行できる。docs production の追加責務分割、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
