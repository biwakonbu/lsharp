# ADR: v0.2 validation manifest review provenance の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 JSON manifest `reviews` registry
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-validation-review-unicode-whitespace.md`

## Context

version 1 manifest の optional `reviews` registry は opaque な `provenance_digest` を保持する。canonical
`ReviewRecord` は `str::trim().is_empty()` で空 digest を拒否するが、manifest input から registry へ
登録する経路で Unicode White_Space-only の digest が fail-closed になることを固定していなかった。
この差が残ると、source review と manifest review で provenance の必須性がずれる。

## Decision

- manifest の `reviews[].provenance_digest` は既存の `IntentGraph::add_review` validation を通し、
  Unicode White_Space-only の値を `ValidationInputError::Graph(GraphError::InvalidReview)` として拒否する。
- error field は canonical の `review_provenance_digest` を保持し、review registry へ部分登録しない。
- review visibility、lifecycle/authentication、source diagnostic code/span、selfhost/native manifest parser、
  CLI/MCP surface はこの変更で拡張しない。

## Evidence

- `review_registry_rejects_unicode_whitespace_only_provenance_digest_in_manifest_input` を RED として追加し、
  version 1 manifest の digest を NBSP-only に変異させ、`EmptyField` を含む `InvalidReview` へ投影する
  ことを固定した。
- production code を変更せず、既存の canonical `trim()` policy が manifest review registry でも適用される
  ことを focused test で確認した。
- 実行: `rustfmt --edition 2024 --check crates/lsharp-types/tests/review_provenance.rs`
- 実行: `cargo test -p lsharp-types --test review_provenance review_registry_rejects_unicode_whitespace_only_provenance_digest_in_manifest_input -- --nocapture`

## Boundary and follow-up

これは Rust canonical manifest review provenance の Unicode non-blank policy に限定した verified
partial slice である。review lifecycle/authentication、manifest の node/coverage parity、selfhost/native
manifest parser、CLI/MCP report parity、current-source stage0 artifact/runtime、Mac Apple Silicon /
Linux x86_64 artifact matrix、EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
