# ADR: selfhost bootstrap four-layer E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer.rs`
- Related: `EC-M2-01`, `EC-M2-02`, `EC-M2-03`, `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

`selfhost_bootstrap_four_layer.rs` は stage1 → stage2 → stage3 の bootstrap、Wasm
runtime import、module/cache resolution、診断用 repro を一つの E2E integration-test
module に集約し、11,779 行まで肥大化していた。bootstrap の observable contract と
fixture を変えずに、failure boundary ごとのレビュー・再実行範囲を小さくする必要がある。

## Decision

- root file は静的な `include!` fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、17 fragment は各 800 行以下に収める。
- test function、helper、fixture、`#[ignore]`、assertion、stage chain / Wasm runtime の
  observable contract は変更しない。
- 分割前後の source SHA-256 と top-level function inventory を比較し、移動のみを機械的に
  検証する。
- size contract は root manifest と全 fragment が 800 行以下で、manifest が全 fragment を
  順序通り include することを固定する。

## Evidence

- RED: size contract を先に追加し、
  `cargo test -p lsharp-wasm --test e2e selfhost_bootstrap_four_layer_source_stays_within_file_size_budget -- --nocapture`
  が `11,779 行`で失敗した。
- GREEN: 同じ size contract は root と17 fragmentで passした。
- `cargo check -p lsharp-wasm --test e2e` は passした。
- `cargo test -p lsharp-wasm --test e2e selfhost_bootstrap_four_layer -- --nocapture` は
  5 passed / 146 ignored / 0 failed（既存 bootstrap module と size contract）だった。
- 分割前後の source SHA-256 と top-level `fn` inventory は一致し、`git diff --check` も
  passした。
- `cargo clippy -p lsharp-wasm --test e2e -- -D warnings` は、今回未変更の
  `selfhost_native_stage_chain.rs` 2件と `support.rs` 1件の既存 lint で失敗した。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。M2 と `LEGACY-MAINT-01` の aggregate は
  `[~]` のまま継続する。

## Consequences

root は17行、fragment は409〜749行となり、bootstrap four-layer の runtime/import/cache
failure を fragment 単位でレビュー・再実行できる。public API、test名、fixture、
bootstrap/Wasm semantics は変わらず、EC-M2 または Rust-free 完了を意味しない。
