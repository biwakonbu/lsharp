# ADR: `tracker.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-docs/src/tracker.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`tracker.rs` はドキュメント追跡状態、AST/doc hash、鮮度更新の production code と、9 件のハッシュ・レビュー状態回帰テストを同じファイルに保持していた。test-only fixture を分離すると、ドキュメント追跡 production の変更と状態 fixture の ownership/review 境界を明確にできる。

## Decision

- `DocStatus`、`DocEntry`、`Freshness`、hash/freshness API の公開契約と semantics は変更しない。
- `#[cfg(test)] mod tests` の 9 件を `crates/lsharp-docs/src/tracker_tests.rs` へ移動する。
- `tracker.rs` は `#[cfg(test)] #[path = "tracker_tests.rs"] mod tests;` で既存の `tracker::tests` namespace を維持する。
- ドキュメント追跡 runtime、serde schema、review integration は同一コミットで変更しない。

## Evidence

- 分離前後の `tracker::tests` focused gate: 9 passed。
- `cargo test -p lsharp-docs`: 23 passed、doc-tests 0 passed。
- `tracker.rs` は 280 行から 135 行へ、`tracker_tests.rs` は 140 行となった。
- `cargo clippy -p lsharp-docs --all-targets -- -D warnings`、対象 files の Rust 2024 rustfmt check、`git diff --check`: pass。

## Consequences

ドキュメント追跡 production と hash/freshness fixture の ownership/review 境界が明確になり、9 件の回帰テストを単独で再実行できる。review integration、他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
