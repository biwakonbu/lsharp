# ADR: v0.2 selfhost source intent graph adapter

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `selfhost/src/Tools/Validation/IntentSource.ls`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`
- Related: `EC-M2-01`, `EC-M2-02`, `docs/development/planning/v0.2-validation-model.md`

## Context

`Parser.ls` は M2 source metadata を directive 順のまま保持できるようになったが、parser の
ordered form を selfhost 側の graph producer が消費する境界がなかった。Rust の
`validation_source::source_program_to_intent_graph` と同じく、node を先に登録してから edge
endpoint を解決し、欠落を成功として隠さない producer が必要である。

## Decision

`Tools.Validation.IntentSource` に、parsed program の defn metadata を tagged vector graph へ
投影する純粋な adapter を追加する。

- node record は `[kind, stable-id, text, span-start, span-end]` とする。
- edge record は `[relation, left-id, right-id, span-start, span-end]` とする。
- graph は `[nodes, edges]`、結果は `[status, graph-or-error]` とする。
- error record は `[code, form-kind, offending-id, span-start, span-end, related-start, related-end]`
  とし、現在の directive span を常に保持する。duplicate node だけは related span に最初の宣言を
  入れ、関連 span がない error は `-1/-1` とする。
- parser AST の module/private/impl body を宣言順に再帰走査し、node を全て登録してから edge を解決する。
- node は Intent / Claim / Assumption / OpenQuestion の wire prefix と本文を検証する。
- edge は `motivates`、`constrained-by`、`tested-by` の typed endpoint を検証する。
- tagged node/edge form 自体は `[kind, payload, start, end]` の4要素に限定し、node payload は
  stable ID と本文、edge payload は endpoint ID の2要素に限定する。内部 tagged vector に余分な
  値があっても payload/form を黙って切り捨てず、directive span 付き malformed error とする。
- tagged form が4要素未満の場合も、存在する kind と開始/終了 offset を診断へ引き継ぐ。
  欠落した offset は `-1` とし、部分的な内部 form を kind `0`・spanなしへ潰さない。
- duplicate node、stable ID の kind mismatch / invalid format、未登録 graph-owned endpoint は
  error record を返し、graph を成功扱いにしない。
- `supports` / `contradicts` は evidence registry がまだ selfhost にないため、
  `evidence-registry-required` 相当の明示 error とする。

source form の順序と directive span は record に保持し、後続の manifest/report projection が
別の順序や暗黙の ID を導入しないようにする。

## Evidence

- `test_e2e_selfhost_source_adapter_projects_nodes_and_edges`: 4 node / 3 edge の ordered projection と node/edge span
- `test_e2e_selfhost_source_adapter_walks_nested_declarations`: nested module/private/impl の 3 node / 1 edge
- `test_e2e_selfhost_source_adapter_rejects_kind_mismatch`
- `test_e2e_selfhost_source_adapter_rejects_invalid_stable_id`
- `test_e2e_selfhost_source_adapter_rejects_duplicate_nodes`
- `test_e2e_selfhost_source_adapter_rejects_missing_edge_node`
- `test_e2e_selfhost_source_adapter_rejects_extra_edge_payload`
- `test_e2e_selfhost_source_adapter_rejects_extra_node_payload`
- `test_e2e_selfhost_source_adapter_rejects_extra_form_fields`
- `test_e2e_selfhost_source_adapter_preserves_partial_malformed_form_context`
- `test_e2e_selfhost_source_adapter_rejects_unregistered_evidence_edge`
- `test_e2e_selfhost_source_adapter_reports_error_spans`: duplicate の first/current span と orphan edge span
- `cargo test -p lsharp-wasm --test e2e selfhost_intent_source_adapter -- --nocapture`

## Residual risk and boundary

これは selfhost producer の verified slice であり、`lsharp validate` command、JSON manifest / report、
`:evidence` record parser、MCP/EmbeddedCli wiring、native stage0、Linux x86_64 parity を完了扱いに
しない。Rust adapter との full differential test と evidence registry projection は M2-02/M2-03 の
後続 RED とする。
