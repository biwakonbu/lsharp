# ADR: selfhost typeinfer E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e_selfhost_typeinfer.rs`
- Related: `LEGACY-MAINT-01`, `EC-M2-01`, `EC-M2-02`, `EC-M2-03`, `I-01`, `I-08`

## Context

`e2e_selfhost_typeinfer.rs` は selfhost の AST/lexer/parser 連携、型推論、record/ADT、
pattern match、computation expression、型エラー診断を一つの integration target に集約し、
5,435 行まで肥大化していた。テストの observable contract と fixture を変えずに、責務と
failure boundary ごとのレビュー・再実行範囲を小さくする必要がある。

## Decision

- root file は module 宣言と静的な `include!` fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、8 fragment は各 800 行以下に収める。
- test function、helper、fixture、assertion、型推論・診断の observable contract は変更しない。
- fragment 間の境界空行だけを除去し、分割前後の top-level `fn` inventory と item source
  sequence を比較して移動のみを確認する。
- size contract は root manifest と全 fragment が 800 行以下で、manifest が全 fragment を
  順序通り include することを固定する。

## Evidence

- RED: size contract を先に追加し、
  `cargo test -p lsharp-wasm --test e2e_selfhost_typeinfer_file_size -- --nocapture`
  が元ファイルの `5,435 行`で失敗した。
- GREEN: size contract は root と8 fragmentで passした。
- `cargo test -p lsharp-wasm --test e2e_selfhost_typeinfer -- --nocapture` は
  87 passed / 0 failed / 0 ignored だった。
- 分割前後の top-level `fn` inventory と item source sequence は一致し、8 fragment の
  Rust 2024 rustfmt、focused clippy、`git diff --check`、docs audit は passした。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。M2 と `LEGACY-MAINT-01` は `[~]` のまま継続する。

## Consequences

root は13行、fragment は395〜738行となり、typeinfer の runtime fixture、正常系、match/
computation、診断ケースを fragment 単位でレビュー・再実行できる。public API、test名、fixture、
型推論 semantics は変わらず、Rust-free 完了や両対応 target の全 parity を意味しない。
