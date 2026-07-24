# ADR: `closure.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-ir/src/closure.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`closure.rs` は自由変数解析の production code と、10 件の AST 回帰テストを同じファイルに保持していた。test-only fixture を分離すると、自由変数解析の production 変更と AST construction の test ownership/review 境界を明確にできる。

## Decision

- `free_variables`、`collect_free_vars`、`collect_pattern_bindings` の API と解析 semantics は変更しない。
- `#[cfg(test)] mod tests` の 10 件と `Span` fixture helper を `crates/lsharp-ir/src/closure_tests.rs` へ移動する。
- `closure.rs` は `#[cfg(test)] #[path = "closure_tests.rs"] mod tests;` で既存の `closure::tests` namespace を維持する。
- 自由変数解析、lowering、module graph の production semantics は同一コミットで変更しない。

## Evidence

- 分離前後の `closure::tests` focused gate: 10 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir`: 257 passed、doc-tests 0 passed。
- `closure.rs` は 304 行から 140 行へ、`closure_tests.rs` は 164 行となった。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-ir --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`: pass。

## Consequences

自由変数解析 production と AST test fixture の ownership/review 境界が明確になり、10 件の回帰テストを単独で再実行できる。lowering/module graph を含む他の大規模 Rust file と I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
