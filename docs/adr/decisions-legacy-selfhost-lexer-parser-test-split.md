# ADR: selfhost lexer/parser parity E2E test seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_lexer_parser.rs`
- Related: `LEGACY-MAINT-01`, `EC-M1-01`, `EC-M1-02`, `I-01`, `I-08`

## Context

`selfhost_lexer_parser.rs` は metadata checker、typed signature、type alias、record/ADT/GADT、
lexer、full S-expression parser の parity E2E を一つの target に集約し、1,607 行まで肥大化していた。
parser/inference/compiler の failure boundary ごとのレビュー・再実行範囲を小さくしつつ、既存の
selfhost runtime fixture と observable contract を保つ必要がある。

## Decision

- root file は `support` import と静的な `include!` fragment manifest だけを保持する。
- top-level item の境界でのみ分割し、3 fragment は各 800 行以下に収める。
- test function、helper、fixture、assertion、parser/type-inference/compiler semantics は変更しない。
- 分割前後の top-level `fn` inventory と item source sequence を比較し、移動と必要な formatting のみを確認する。
- size contract は root manifest と全 fragment が 800 行以下で、manifest が全 fragment を順序通り include することを固定する。

## Evidence

- RED: size contract を先に追加し、
  `cargo test -p lsharp-wasm --test e2e_selfhost_lexer_parser_file_size -- --nocapture`
  が元ファイルの `1,607 行`で失敗した。
- GREEN: 分割後の size contract は passした。
- `cargo test -p lsharp-wasm --test e2e selfhost_lexer_parser -- --nocapture` は
  32 passed / 0 failed / 2,818 filtered だった（別セッションの重い E2E と同時実行中のため 378.38 秒）。
- 分割後の `--no-run` compile、top-level `fn` inventory、item source sequence、Rust 2024
  rustfmt、`git diff --check` は passした。
- `cargo clippy -p lsharp-wasm --test e2e -- -D warnings` は今回未変更の
  `selfhost_native_stage_chain.rs` 2件と `support.rs` 1件の既存 lint で失敗した。
- 対象 module は test-only の構造変更であり、native stage0、Linux VM、release artifact の
  新しい parity evidence は追加していない。M1 と `LEGACY-MAINT-01` は `[~]` のまま継続する。

## Consequences

root は5行、fragment は429〜624行となり、metadata/type inference/compiler、lexer、parser の
責務ごとにレビュー・再実行できる。public API、test名、fixture、parser semantics は変わらず、
Rust-free 完了や両対応 target の全 parity を意味しない。
