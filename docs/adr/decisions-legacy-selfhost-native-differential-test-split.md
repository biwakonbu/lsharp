# ADR: selfhost native differential E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_native_differential.rs`
- Related: `EC-M2-01`, `EC-M2-02`, `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

`selfhost_native_differential.rs` は Wasm/native の構造差分、native emitter/linker の
target 別 byte contract、standalone pipeline smoke を一つの integration-test module に
集約し、12,620 行まで肥大化していた。テストの意味論を変えずに failure boundary ごとの
再実行とレビュー範囲を小さくする必要がある。

## Decision

- root file は静的 `include!` の fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、19 fragment は各 800 行以下に収める。
- test function、helper、fixture、`#[ignore]`、assertion、native/Wasm の observable contract
  は変更しない。
- source fragment の SHA-256 と top-level function inventory を分割前後で比較し、移動のみを
  機械的に検証する。
- size contract は root と全 fragment が 800 行以下であることを固定する。

## Evidence

- RED: 先に size contract を追加し、
  `cargo test -p lsharp-wasm --test e2e selfhost_native_differential_source_stays_within_file_size_budget -- --nocapture`
  が `12,620 行`で失敗した。
- GREEN: 同じ size contract は root と19 fragmentで passした。
- `cargo check -p lsharp-wasm --test e2e` は passした。
- `cargo test -p lsharp-wasm --test e2e selfhost_native_differential -- --nocapture` は
  17 passed / 104 ignored / 0 failed（既存 native differential tests）だった。
- 分割前後の top-level `fn` inventory と source SHA-256 は一致し、`git diff --check` も passした。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。M2 と `LEGACY-MAINT-01` の aggregate は `[~]` のまま継続する。
- e2e 全体の rustfmt check は、今回未変更の既存 e2e files の formatting 差分を含むため
  clean ではない。対象 module の compile/test と size/diff contract を採用 gate とする。

## Consequences

root は 19 行、fragment は 475〜747 行（一部の末尾 fragment は 140 行）となり、native
differential の target/arity failure を fragment 単位で再実行できる。public API、test名、
native/Wasm semantics は変わらず、M2 の intent/evidence graph または Rust-free 完了を意味しない。
