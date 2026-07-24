# ADR: `resolver.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/resolver.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`resolver.rs` は semver 要求、cache package 探索、依存解決の production code と、VersionReq の compatible/exact/minimum 判定および cache 候補選択の 4 件の回帰テストを同じファイルに保持していた。test-only fixture を分離すると、resolver production の変更と version-selection fixture の ownership/review 境界を明確にできる。

## Decision

- `SemVersion`、`VersionReq`、cache 探索・選択・依存解決 API と semantics は変更しない。
- `#[cfg(test)] mod tests` の 4 件を `crates/lsharp-driver/src/resolver_tests.rs` へ移動する。
- `resolver.rs` は `#[cfg(test)] #[path = "resolver_tests.rs"] mod tests;` で既存の `resolver::tests` namespace を維持する。
- resolver runtime、Config/package cache、CLI install 経路は同一コミットで変更しない。

## Evidence

- 分離前後の `resolver::tests` focused gate: 4 passed。
- `cargo test -p lsharp-driver --bin lsharp`: 132 passed。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings`、Rust 2024 rustfmt、`git diff --check`: pass。
- `resolver.rs` は 233 行から 184 行へ、`resolver_tests.rs` は 49 行となった。
- `bash scripts/audit_docs.sh`: エラー 0、警告 0。

## Consequences

resolver production と version-selection fixture の ownership/review 境界が明確になり、4 件の回帰テストを単独で再実行できる。driver の embedded component integration、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
