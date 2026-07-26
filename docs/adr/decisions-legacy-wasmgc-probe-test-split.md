# ADR: WasmGC probe integration-test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/wasmgc_probe.rs`
- Related: `LEGACY-EXEC-01`, `LEGACY-MAINT-01`, `I-01`, `I-08`

## Context

`wasmgc_probe.rs` は WasmGC emitter、Preview2/component runner、WASI CLI stream、filesystem
descriptor、funcref と error propagation の integration tests を一つの test target に
集約し、10,704 行まで肥大化していた。WasmGC の runtime contract と fixture を変えずに、
failure boundary ごとのレビュー・再実行範囲を小さくする必要がある。

## Decision

- root file は静的な `include!` fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、16 fragment は各 800 行以下に収める。
- test function、helper、fixture、assertion、WasmGC/component/WASI の observable contract
  は変更しない。
- fragment 間の境界空行だけを除去して各 fragment を Rust 2024 rustfmt の入力として成立させ、
  top-level `fn` inventory と実行テストで item/semantics の移動のみを確認する。
- size contract は root manifest と全 fragment が 800 行以下で、manifest が全 fragment を
  順序通り include することを固定する。

## Evidence

- RED: size contract を先に追加し、
  `cargo test -p lsharp-wasm --test wasmgc_probe_file_size wasmgc_probe_source_stays_within_file_size_budget -- --nocapture`
  が `10,704 行`で失敗した。
- GREEN: 同じ size contract は root と16 fragmentで passした。
- `cargo check -p lsharp-wasm --test wasmgc_probe` は passした。
- `cargo test -p lsharp-wasm --test wasmgc_probe -- --nocapture` は
  101 passed / 0 failed だった。
- 分割前後の top-level `fn` inventory は一致し、fragment 全体の Rust 2024 rustfmt check と
  `git diff --check` も passした。変更した境界空行以外の source item/fixture は移動のみである。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。`LEGACY-EXEC-01` と `LEGACY-MAINT-01` は
  `[~]` のまま継続する。

## Consequences

root は16行、fragment は314〜747行となり、WasmGC の emitter、component runner、filesystem
descriptor、stream failure を fragment 単位でレビュー・再実行できる。public API、test名、
fixture、WasmGC/WASI semantics は変わらず、Rust-free 完了や Preview2 の全 target 対応を意味しない。
