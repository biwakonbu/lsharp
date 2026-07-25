# ADR: v0.2 validation manifest の duplicate evidence ID 境界

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_input.rs` の evidence 配列入力
- Related: `EC-M2-02`, `docs/adr/decisions-v0.2-validation-input-parser.md`,
  `docs/adr/decisions-v0.2-selfhost-evidence-registry.md`

## Context

`EvidenceGraph::add_evidence` は同じ stable ID の二重登録を拒否するが、JSON manifest の
evidence 配列から parser を通る duplicate ID の failure boundary が専用テストに現れていなかった。
入力順序によって同じ evidence が上書き・黙殺されると、後続 edge がどの execution/provenance を
参照するか不定になる。

## Decision

- manifest の evidence は canonical `EvidenceId` を graph へ順に登録し、同一 namespace/key が二度
  現れた時点で `ValidationInputError::Graph(GraphError::DuplicateEvidence { .. })` を返す。
- duplicate を merge、last-write-wins、または warning-only にしない。parser は成功 graph を返さず、
  edge parsing へ進めない。
- duplicate identity の判定は typed graph の stable ID policy に委譲し、method/outcome/provenance の
  差分を別 evidence として扱わない。

## Evidence

- `parse_manifest_rejects_duplicate_evidence_ids` は同じ `evidence:checkout/review-001` を持つ2件の
  manifest inputを読み、typed `GraphError::DuplicateEvidence` と exact ID を検証する。
- RED のテスト追加後、production code を変更せず focused test が GREEN になった。既存 graph
  invariant が JSON input boundary まで到達することを契約化した。
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_duplicate_evidence_ids -- --nocapture`

## Boundary

これは Rust manifest input の duplicate evidence identity に限定した verified slice である。
source syntax registry、selfhost/native stage0、CLI/MCP report/exit parity、duplicate source span の
diagnostic、Mac Apple Silicon / Linux x86_64 artifact/runtime evidence、EC-M2 aggregate の完了を
意味しない。
