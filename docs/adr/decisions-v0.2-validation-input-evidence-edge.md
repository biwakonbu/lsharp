# ADR: v0.2 validation manifest の evidence edge closure

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_input.rs` の edge decoder
- Related: `EC-M2-02`, `EC-M2-03`, `docs/adr/decisions-v0.2-validation-input-parser.md`

## Context

manifest の edge は evidence registry と node graph を同じ入力で参照する。typed graph は
`supports` / `contradicts` / `evaluates` / `invalidates` が存在しない evidence ID を拒否するが、
JSON parser の `evaluates` subject がその closure boundary に到達することを専用テストで固定して
いなかった。未知 evidence を黙って edge に残すと、後続 validation report が実体のない証跡を
assurance として扱う可能性がある。

## Decision

- manifest の evidence を先に graph へ登録し、evidence を参照する edge は登録済み ID だけを受理する。
- `evaluates` の `subject.kind = evidence` が未知 ID を参照した場合、
  `ValidationInputError::Graph(GraphError::MissingEvidence { .. })` を返し、成功 graph を返さない。
- edge の順序や graph-only の review identity は保持するが、referential closure を warning-only や
  後段の report 判定へ遅延しない。

## Evidence

- `parse_manifest_rejects_edges_that_reference_missing_evidence` は complete manifest の
  `evaluates` subject だけを `evidence:checkout/missing-evidence` に変え、typed
  `GraphError::MissingEvidence` と exact ID を検証する。
- RED のテスト追加後、production code を変更せず focused test が GREEN になった。既存 graph
  edge closure invariant が JSON input boundary でも維持されることを契約化した。
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_edges_that_reference_missing_evidence -- --nocapture`

## Boundary

これは Rust manifest input の evidence edge closure に限定した verified slice である。source
syntax registry、selfhost/native stage0、CLI/MCP report/exit parity、全 edge relation の native
artifact/runtime evidence、Mac Apple Silicon / Linux x86_64 gate、EC-M2 aggregate の完了を意味しない。
