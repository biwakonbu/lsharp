# ADR: selfhost native stage23 gap E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_native_stage23_gap.rs`
- Related: `LEGACY-EXEC-01`, `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

`selfhost_native_stage23_gap.rs` は native x86_64 / aarch64 の helper bytes、stack-depth、
call-site offset、bounded heap、selfhost opcode mapping を一つの E2E test module に集約し、
2,341 行まで肥大化していた。native codegen の byte contract と fixture を変えずに、target・
helper・runtime failure boundary ごとのレビュー・再実行範囲を小さくする必要がある。

## Decision

- root file は静的な `include!` fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、4 fragment は各 800 行以下に収める。
- test function、helper、fixture、assertion、x86_64 / aarch64 の observable byte contract は
  変更しない。
- fragment 間の境界空行だけを除去し、分割前後の top-level `fn` inventory と item source
  sequence を比較して移動のみを確認する。
- size contract は root manifest と全 fragment が 800 行以下で、manifest が全 fragment を
  順序通り include することを固定する。

## Evidence

- RED: size contract を先に追加し、
  `cargo test -p lsharp-wasm --test e2e selfhost_native_stage23_gap_source_stays_within_file_size_budget -- --nocapture`
  が `2,341 行`で失敗した。
- GREEN: size contract は root と4 fragmentで passした。
- `cargo test -p lsharp-wasm --test e2e selfhost_native_stage23_gap -- --nocapture` は
  32 passed / 10 failed / 3 ignored だった。失敗は既存の helper byte offset/depth 期待値と
  selfhost fixture の `Syntax.AST` 解決境界であり、今回変更した source item / fixture の
  意味論ではない。分割前後の item sequence は同一で、差分は3箇所の境界空行除去と manifest
  化だけである。
- fragment の Rust 2024 rustfmt は元 source に既存の array formatting 差分を含むため clean
  ではない。新規 size contract の rustfmt、`git diff --check`、manifest/size contract は passした。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。`LEGACY-EXEC-01` と `LEGACY-MAINT-01` は
  `[~]` のまま継続する。

## Consequences

root は4行、fragment は317〜730行となり、native helper/call-site/stack failure を fragment
単位でレビュー・再実行できる。public API、test名、fixture、native byte semantics は変わらず、
Rust-free 完了や両対応 target の全 parity を意味しない。既存10 failure は別の実装修正タスクへ
残る。
