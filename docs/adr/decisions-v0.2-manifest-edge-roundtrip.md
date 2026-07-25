# ADR: v0.2 manifest の全 edge variant round-trip

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/{validation_input,validation_output}.rs`
- Related: `EC-M2-03`, `decisions-v0.2-validation-input-parser.md`,
  `decisions-v0.2-validation-types-slice.md`

## Context

version 1 の graph manifest は `motivates`、`constrained-by`、`tested-by`、`supports`、
`contradicts`、`evaluates`、`invalidates` の7種類の edge wire を持つ。既存テストは
出力側で relation 名を確認し、入力側では代表的な3 edge の round-trip を確認していたが、
出力 serializer と入力 parser が全 variant で対称であることは固定されていなかった。

## Decision

- `IntentGraph` の全 edge variant を含む fixture を canonical manifest JSON へ出力する。
- その JSON を `parse_intent_graph_json` へ戻し、graph の node/evidence/edge と validation
  facts が完全一致することを契約とする。
- この契約は graph の登録順を保持する現行 wire policy を変更せず、serializer と parser の
  variant coverage だけを閉じる。

## Evidence

- Contract test: `manifest_output_round_trips_all_edge_variants` を追加し、全7 relation と
  `evaluates` の subject 2種、`invalidates` の evidence subject を同じ fixture で検証。
- GREEN: `cargo test -p lsharp-types --test validation_output manifest_output_round_trips_all_edge_variants -- --nocapture`
  が pass。

## Boundary

これは Rust `lsharp-types` の manifest serializer/parser 間の wire parity に限定した
verified slice である。source syntax、CLI/native stage0、selfhost parity、release
provenance、M2 aggregate の完了を意味しない。
