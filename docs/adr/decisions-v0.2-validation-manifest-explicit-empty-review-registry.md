# ADR: v0.2 validation manifest の明示 empty review registry

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: version 1 JSON manifest の `reviews` registry と typed review edge closure
- Related: `EC-M2-02`、`EC-M2-03`、`EC-M3-01`

## Context

`reviews` を省略した manifest は外部 review registry を持たないため、review identity を
opaque endpoint として扱える。一方、`reviews: []` を明示した manifest も内部では空の
`Vec` に変換されていたため、未登録 review edge が registry なしと同じように通過していた。
これは「registry が明示された入力では endpoint closure を検査する」という契約に反し、
空 registry を fail-open にする。

## Decision

- `Manifest.reviews` は `Option<Vec<ReviewInput>>` とし、省略 (`None`) と明示 empty (`Some([])`)
  を区別する。
- `IntentGraph` は review registry の明示性を保持し、明示された registry では空でも
  `evaluates` / `invalidates` の未登録 review を `GraphError::MissingReview` で拒否する。
- canonical manifest output は明示 empty registry を `"reviews": []` として保持し、parse→emit→parse
  で closure policy を失わない。省略された registry は従来どおり field を出力しない。

## Evidence

- RED: `explicit_empty_review_registry_rejects_unregistered_review_edges` は `reviews: []` の
  未登録 `evaluates` edge が従来受理されることを確認して失敗した。
- GREEN: 同 fixture が `ValidationInputError::Graph(GraphError::MissingReview)` で拒否されることを固定した。
- `explicit_empty_review_registry_round_trips_as_an_empty_array` で empty registry の wire presence と
  parse/emit roundtrip を固定した。
- focused `review_provenance`、`validation_output`、`validation_source` tests が pass。

## Boundary and follow-up

これは Rust canonical graph/manifest input-output の review registry presence boundary に限定した
verified partial slice である。review lifecycle/authentication、selfhost/native manifest parser、
CLI/MCP parity、current-source stage0 artifact/runtime、Mac Apple Silicon / Linux x86_64 matrix、
EC-M2-02 / EC-M2-03 / EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
