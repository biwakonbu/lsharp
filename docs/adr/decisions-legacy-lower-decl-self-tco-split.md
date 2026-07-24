# ADR: `lower/decl.rs` Self-TCO helper 分離
- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-ir/src/lower/decl.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`, `decisions-legacy-lower-test-module-split.md`

## Context

`lower/decl.rs` は宣言・関数 lowering と Self-TCO の命令変換 helper を同じファイルに保持し、823 行になっていた。Self-TCO は root slot、末尾 call 検出、loop/backedge 変換という独立した責務であり、宣言 lowering の差分と分けてレビューできる。

## Decision

- 宣言 lowering の公開 API、Self-TCO の意味論、既存 test fixture は変更しない。
- `lower/decl.rs` から `SelfTcoRootOps`、`apply_self_tco`、`find_simple_self_tail_calls` を `lower/decl/self_tco.rs` へ移す。
- `decl.rs` は `self_tco` module を宣言し、parent-visible helper として呼び出す。親は 692 行、child は 139 行とし、既存の `lower::tests` namespace を維持する。
- lower expr/mod の production split、I-01 / I-08 aggregate、selfhost/native parity は別タスクとして残す。

## Evidence

- 分離前後の Self-TCO focused gate `CARGO_TARGET_DIR=... cargo test -p lsharp-ir lower::tests::rooting_loops -- --nocapture`: 8 passed。
- 分離後の lower focused gate `cargo test -p lsharp-ir lower::tests -- --nocapture`: 143 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 257 passed。
- `RUST_MIN_STACK=33554432 cargo clippy -p lsharp-ir --all-targets -- -D warnings`、対象 files の rustfmt check、`git diff --check`: pass。
- `decl.rs` は 823 行から 692 行へ、`decl/self_tco.rs` は 139 行となった。production semantics の差分は helper の module 移動だけである。
- default stack の Formatter incremental fixture overflow は既知の failure boundary であり、large-stack gate の pass と分離して扱う。

## Consequences

宣言 lowering と Self-TCO の ownership/review 境界が明確になり、Self-TCO の focused rerun が可能になった。lower expr/mod や他の大規模 Rust file、I-01 / I-08 aggregate は未完了であるため、TODO の partial slice を維持する。
