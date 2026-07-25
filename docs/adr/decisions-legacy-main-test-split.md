# ADR: `lsharp-driver/main.rs` inline test 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-driver/src/main.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`

## Context

`main.rs` は CLI 定義・driver dispatch・package operations に加えて、test-only の import visibility
resolver、Knowledge fixture builder、git clone argument helper、132 件の inline test を同居させていた。
test-only 部分を分離すると、CLI production と command/integration fixture の ownership と review 境界を明確にできる。

## Decision

- CLI parser、command dispatch、package/install、embedded component delegation の公開 surface と
  production semantics は変更しない。
- test-only helper と `#[cfg(test)] mod tests` の 132 件を
  `crates/lsharp-driver/src/main_tests.rs` へ移動する。
- `main.rs` は `#[cfg(test)] #[path = "main_tests.rs"] mod tests;` で従来の
  `tests::*` namespace と private item access を維持する。
- test body、fixture、assertion は変更しない。embedded component / selfhost artifact integration
  failure は別タスクの既存 failure boundary として残す。

## Evidence

- 分離前後の `cargo test -p lsharp-driver tests -- --nocapture`: 132 passed。
- `main.rs` は 4715 行から 2438 行へ、`main_tests.rs` は 2271 行となった。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-driver`: unit 132 passed、
  `default_path_delegation` は 34 passed / 12 failed。失敗は embedded component / selfhost artifact
  の既存境界（summary shape、Preview1 runtime output、`build-wasm-bytes-wasi` 未定義など）であり、
  test-only 移動とは無関係。
- `cargo clippy -p lsharp-driver --all-targets -- -D warnings` は pass。
- 対象2ファイルの Rust 2024 rustfmt、`git diff --check` は pass。全 workspace の `cargo fmt --all -- --check`
  は今回の対象外ファイルにも既存の formatting 差分があるため、対象ファイルの gate とは分離した。

## Consequences

CLI production と driver regression fixture の ownership/review 境界が明確になり、132 件の unit test を
単独で再実行できる。`main.rs` の command 単位 production 分割、他の大規模 Rust file 分割、I-01 / I-08
aggregate は未完了であるため、TODO の partial slice を維持する。
