# ADR: v0.2 review invalidation の stale propagation

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `lsharp-types::validation::IntentGraph::stale_subjects`
- Related: `EC-M2-02`、`docs/development/planning/v0.2-evidence-graph.md`

## Context

M2-02 には `EvidenceOutcome::Stale` と `invalidates` edge が存在するが、直接 stale と
review の失効を同じ deterministic な projection へまとめる lifecycle policy が未固定だった。
このまま report consumer ごとに判定すると、同じ graph でも stale evidence の順序や重複除去が
変わり得る。

## Decision

`IntentGraph::stale_subjects()` は次の順序で `StaleSubjects` を構築する。

1. graph に登録された `EvidenceOutcome::Stale` を evidence 登録順で収集する。
2. `Invalidates(Review(id))` と `Invalidates(Evidence(id))` を edge 宣言順で追加する。
3. stale review ごとに、その review の `Evaluates` が直接指す
   `ReviewSubject::Evidence(id)` だけを evidence へ追加する。
4. review/evidence ID は最初の出現順を保って重複を除く。

`Evaluates` の Intent/Claim subject は review の stale を伝播させない。evidence から review
への逆向き推論や provider authentication、署名検証、外部 lifecycle の自動更新はこの API の
責務に含めない。

## Evidence

- RED: `cargo test -p lsharp-types --test review_stale_propagation -- --nocapture` は
  `IntentGraph::stale_subjects` 未実装のため 3 件すべてコンパイル失敗。
- GREEN: 同コマンドで 3 passed。
- Regression: `intent_validation` 6 passed、`evidence_graph` 5 passed、`review_provenance`
  4 passed、`validation_source` 31 passed、`validation_output` 5 passed。
- `bash scripts/audit_docs.sh` は docs 反映後に実行する。

## Boundary

これは Rust canonical graph の deterministic stale projection に限る verified slice である。
selfhost/native stage0 parity、review provider/署名 authentication、外部 lifecycle、MCP/公開
report の stale wire、Mac Apple Silicon / Linux x86_64 runtime evidence は未完了であり、
`EC-M2-02` は TODO の `[~]` を維持する。
