# ADR: `hygiene.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-syntax/src/hygiene.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`hygiene.rs` は Sets of Scopes の production API と 17 件の回帰テストを同じファイルに保持していた。衛生識別子・scope set・binding table の挙動を確認する test-only 差分を分離すると、macro hygiene の production 差分と回帰テストを別々にレビューできる。

## Decision

- `ScopeId`、`ScopeSet`、`HygienicIdent`、`ScopeAllocator`、`HygienicBindingTable` の公開 API と意味論は変更しない。
- `#[cfg(test)] mod tests` の 17 件を `crates/lsharp-syntax/src/hygiene/tests.rs` へ移動する。
- `hygiene.rs` は `#[cfg(test)] mod tests;` で既存の `hygiene::tests` namespace を維持する。
- macro expansion の production split、parser/lexer の production split、I-01 / I-08 aggregate は別タスクとして残す。

## Evidence

- 分離前後の `hygiene::tests` focused gate: 17 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-syntax`: 163 unit tests、integration/property 12 tests passed。
- `hygiene.rs` は 546 行から 297 行へ、`hygiene/tests.rs` は 249 行となった。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-syntax --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`: pass。

## Consequences

macro hygiene の production API と test-only fixture の ownership/review 境界が明確になり、17 件の回帰テストを単独で再実行できる。lexer、parser、macro expansion、その他の大型 production file と I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
