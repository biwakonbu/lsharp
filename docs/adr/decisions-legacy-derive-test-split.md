# ADR: `derive.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-syntax/src/derive.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`derive.rs` は ADT/record derive の production builder と 7 件の AST shape 回帰テストを同じファイルに保持していた。derive の生成 semantics と test fixture を分離すると、追加 derive 対応の差分を小さくレビューできる。

## Decision

- `derive_show_adt`、`derive_eq_adt`、`derive_show_record`、`apply_derives` の API と生成 semantics は変更しない。
- `#[cfg(test)] mod tests` の 7 件を `crates/lsharp-syntax/src/derive/tests.rs` へ移動する。
- `derive.rs` は `#[cfg(test)] mod tests;` で既存の `derive::tests` namespace を維持する。
- parser/lexer production split、他の syntax production split、I-01 / I-08 aggregate は別タスクとして残す。

## Evidence

- 分離前後の `derive::tests` focused gate: 7 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-syntax`: 163 unit tests、integration/property 12 tests passed。
- `derive.rs` は 507 行から 335 行へ、`derive/tests.rs` は 175 行となった。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-syntax --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`: pass。

## Consequences

derive production builder と test-only AST fixture の ownership/review 境界が明確になり、7 件の回帰テストを単独で再実行できる。parser/lexer、macro expansion、その他の大規模 Rust file と I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
