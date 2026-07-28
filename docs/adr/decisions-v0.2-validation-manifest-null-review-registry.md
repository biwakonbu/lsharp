# ADR: v0.2 validation manifest の null review registry 拒否

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: version 1 JSON manifest の optional `reviews` field
- Related: `EC-M2-02`、`EC-M2-03`、`EC-M3-01`

## Context

`reviews` は省略時だけ registry なしを表し、配列が存在する場合は review registry の
closure を有効にする。`Option<Vec<ReviewInput>>` の通常の serde deserialization は
明示された `null` を `None` に変換するため、schema 外の null が省略と同じ fail-open
境界へ流れていた。

## Decision

- `reviews` は省略なら `None`、空を含む JSON 配列なら `Some(Vec<ReviewInput>)` とする。
- 明示 `null` は custom deserializer で `ValidationInputError::Json` として拒否する。
- explicit empty registry の closure/roundtrip policy は既存の `IntentGraph` marker に委ね、
  この sliceでは null の型境界だけを閉じる。

## Evidence

- RED: `manifest_rejects_null_review_registry_instead_of_treating_it_as_absent` は `reviews: null`
  が従来受理されることを確認して失敗した。
- GREEN: 同じ fixture が graph/report 生成前の `ValidationInputError::Json` で拒否される。
- `cargo test -p lsharp-types --test review_provenance`
- `cargo test -p lsharp-types`
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`

## Boundary and follow-up

これは Rust canonical manifest input の optional registry type boundary に限定した verified
partial slice である。selfhost/native parser、CLI/MCP parity、current-source stage0
artifact/runtime、Mac Apple Silicon / Linux x86_64 matrix、review lifecycle/authentication、
EC-M2-02 / EC-M2-03 / EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
