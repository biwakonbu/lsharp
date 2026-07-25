# ADR: `host_bridge.rs` の test/fixture 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-wasm/src/host_bridge.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`host_bridge.rs` は host capability の公開実装と HTTP handler の linker 登録に加えて、
synthetic HTTP state fixture と 7 件の Wasmtime bridge test を同じファイルに保持していた。
ファイルは 1,032 行で、production の変更境界と test fixture の変更境界が混在していた。

## Decision

- production の host capability/linker 実装を `host_bridge.rs` に残し、`#[cfg(test)] mod tests;`
  で子 module を接続する。
- private binding/helper へのアクセスを保つため、`tests/mod.rs` から
  `operations.rs` と `synthetic_http_state.rs` を `include!` する。既存の
  `host_bridge::tests::*` namespace と 7 件の test 名は変更しない。
- production semantics、公開 API、WIT binding、runtime fixture の挙動は変更しない。
- file-size allowlist は origin/main ですでに撤去済みのため、この slice では allowlist を変更しない。
- production 責務の追加分割、full `lsharp-wasm`/native gate、I-01 / I-08 aggregate は後続タスクとする。

## Evidence

- `CARGO_TARGET_DIR=... cargo test -p lsharp-wasm --lib host_bridge`: 7 passed, 0 failed。
- `host_bridge.rs` は 1,032 行から 126 行へ縮小し、fixture/operation test はそれぞれ 595 / 278 行となった。
- 対象 Rust files の Rust 2024 targeted rustfmt、`git diff --check`、docs audit は pass。

## Consequences

host bridge の production/linker 実装と synthetic HTTP fixture/test operation を独立してレビューできる。
private boundary と test namespace は維持されるため利用側の変更は不要である。full runtime parity、
他の大規模 Rust file、I-01 / I-08 aggregate は未完了であり、TODO の verified partial slice を維持する。
