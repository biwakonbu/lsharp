# ADR: selfhost evidence registry E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry.rs`
- Related: `EC-M2-02`, `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

`selfhost_evidence_registry.rs` は selfhost の evidence record 登録、manifest projection、
Rust oracle parity、malformed input と duplicate registry の fail-closed tests を一つの
E2E integration-test target に集約し、773 行まで肥大化していた。M2-02 の意味論と fixture
を変えず、成功経路と拒否経路を責務単位でレビュー・再実行できる構造へ分ける必要がある。

## Decision

- root target はサイズ契約と `#[path]` module tree だけを保持する。
- `harness.rs` は canonical selfhost source bundle の読み込みと runtime 実行 helper を保持する。
- `runtime.rs` は evidence record、source edge、manifest serializer、Rust canonical/oracle の
  成功経路と registry closure tests を保持する。
- `validation.rs` は malformed coverage、payload、duplicate registry の fail-closed tests を保持する。
- test function、fixture、assertion、`Evidence.ls`、`support.rs`、adapter の公開 API、
  selfhost/runtime の意味論は変更しない。
- root の 500 行以下を `selfhost_evidence_registry_root_stays_within_the_test_file_budget`
  で固定する。

## Evidence

- RED: サイズ契約を先に追加し、
  `cargo test -p lsharp-wasm --test e2e selfhost_evidence_registry_root_stays_within_the_test_file_budget -- --nocapture`
  が `actual=789` で失敗。
- GREEN: `cargo test -p lsharp-wasm --test e2e selfhost_evidence_registry -- --nocapture`
  が 16 tests passed（既存 15 tests + サイズ契約）。
- 旧ファイルと新 module の test function inventory は一致し、fixture/assertion は移動のみ。
- 対象ファイルの Rust 2024 rustfmt と `git diff --check` は pass。
- `cargo clippy -p lsharp-wasm --test e2e -- -D warnings` は、今回触れていない
  `selfhost_native_stage_chain.rs` の既存 2件と `support.rs` の既存 1件で失敗した。
  これらは別作業の差分に属するため修正しない。
- 本変更は test-only の責務分割であり、native stage0 の新しい parity evidence は追加しない。
  selfhost E2E の既存 runtime evidence と M2-02 aggregate は `[~]` のまま継続する。

## Consequences

root は 24 行、child modules は `harness.rs` 25 行、`runtime.rs` 396 行、`validation.rs` 360 行
となり、failure boundary ごとの再実行と競合範囲が小さくなる。evidence graph の semantics、
manifest wire shape、Rust/native parity、target 対応範囲は変更せず、EC-M2-02 または
Rust-free 完了を意味しない。
