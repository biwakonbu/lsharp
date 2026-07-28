# ADR: v0.2 validation manifest duplicate review identity boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 JSON manifest `reviews` registry
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-validation-input-duplicate-evidence.md`

## Context

canonical `IntentGraph::add_review` は review ID を registry identity として扱い、同じ ID の二重登録を
`GraphError::DuplicateReview` で拒否する。manifest input では duplicate review record を graph 登録前の
入力契約として列挙しておらず、source/manifest の registry closure を同じ failure boundary として確認
できていなかった。

## Decision

- version 1 manifest の `reviews` は canonical `IntentGraph::add_review` を順に通し、同一の
  `review:namespace/key` identity が二度現れた場合は `ValidationInputError::Graph(GraphError::DuplicateReview)`
  として fail-closed にする。
- 最初の review を保持したまま部分 graph を返さず、後続の digest/visibility を上書きしない。
- review lifecycle/authentication、wire duplicate JSON key policy、selfhost/native manifest parser、CLI/MCP
  surface はこの変更で拡張しない。

## Evidence

- `review_registry_rejects_duplicate_review_ids_in_manifest_input` を RED として追加し、同じ review IDに
  異なる digest/visibilityを持つ2 recordsを version 1 manifestへ入れ、duplicate wire IDを保持して拒否
  することを固定した。
- production codeを変更せず、既存の canonical registry identity policyが manifest inputでも適用される
  ことを focused test で確認した。
- 実行: `rustfmt --edition 2024 --check crates/lsharp-types/tests/review_provenance.rs`
- 実行: `cargo test -p lsharp-types --test review_provenance review_registry_rejects_duplicate_review_ids_in_manifest_input -- --nocapture`

## Boundary and follow-up

これは Rust canonical manifest review identity の duplicate rejection に限定した verified partial slice
である。review provenance authentication/lifecycle、manifest の other duplicate keys、selfhost/native
manifest parser、CLI/MCP report parity、current-source stage0 artifact/runtime、Mac Apple Silicon /
Linux x86_64 artifact matrix、EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
