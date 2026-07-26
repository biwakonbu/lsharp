# ADR: selfhost bootstrap acceptance E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_acceptance.rs`
- Related: `LEGACY-MAINT-01`, `EC-M2-01`, `EC-M2-02`, `EC-M2-03`, `I-01`, `I-08`

## Context

`selfhost_bootstrap_acceptance.rs` は stage1→stage2 / stage2→stage3 fixed-point、section・
symbol stability、fixed input set、CLI/LSP module の acceptance probes を一つの E2E target
に集約し、2,457 行まで肥大化していた。bootstrap acceptance contract と fixture を変えずに、
probe/fixed-point failure boundary ごとのレビュー・再実行範囲を小さくする必要がある。

## Decision

- root file は静的な `include!` fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、4 fragment は各 800 行以下に収める。
- test function、helper、fixture、assertion、stage-chain の observable contract は変更しない。
- fragment 間の境界空行だけを除去し、分割前後の top-level `fn` inventory と item source
  sequence を比較して移動のみを確認する。
- size contract は root manifest と全 fragment が 800 行以下で、manifest が全 fragment を
  順序通り include することを固定する。

## Evidence

- RED: size contract を先に追加し、
  `cargo test -p lsharp-wasm --test e2e selfhost_bootstrap_acceptance_source_stays_within_file_size_budget -- --nocapture`
  が `2,457 行`で失敗した。
- GREEN: size contract は root と4 fragmentで passした。
- `cargo test -p lsharp-wasm --test e2e selfhost_bootstrap_acceptance -- --nocapture` は
  3 passed / 28 ignored / 0 failed だった。
- 分割前後の top-level `fn` inventory と item source sequence は一致し、4 fragment の
  Rust 2024 rustfmt、size contract、`git diff --check` は passした。
- `cargo clippy -p lsharp-wasm --test e2e -- -D warnings` は今回未変更の
  `selfhost_native_stage_chain.rs` 2件と `support.rs` 1件の既存 lint で失敗した。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。M2 と `LEGACY-MAINT-01` は `[~]` のまま継続する。

## Consequences

root は4行、fragment は309〜740行となり、bootstrap acceptance の fixed-point、probe、
module-specific failure を fragment 単位でレビュー・再実行できる。public API、test名、fixture、
bootstrap semantics は変わらず、Rust-free 完了や両対応 target の全 parity を意味しない。
