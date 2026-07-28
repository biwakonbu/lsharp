# ADR: v0.2 validation manifest の typed subject schema

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `docs/schemas/intent-graph.schema.json` の evidence/edge subject definitions
- Related: `EC-M2-02`、`EC-M2-03`、`EC-M3-01`

## Context

Rust の version 1 manifest parser は subject の利用先ごとに kind を固定している。
evidence は `intent` / `claim` / `contract`、`evaluates` は `intent` / `claim` /
`evidence`、`invalidates` は `evidence` / `review` だけを受理する。一方、公開 JSON Schema
は全ての subject に汎用 enum を参照していたため、schema consumer が Rust parser で拒否される
`contract` の `evaluates` や `claim` の `invalidates` を受理できた。

## Decision

- evidence の `subject` は `evidence-subject` 定義へ参照し、kind を `intent` / `claim` /
  `contract` に限定する。
- `evaluates` の `subject` は `review-subject` 定義へ参照し、kind を `intent` / `claim` /
  `evidence` に限定する。
- `invalidates` の `subject` は `invalidation-subject` 定義へ参照し、kind を `evidence` /
  `review` に限定する。
- relation ごとの subject kind を schema に明示し、Rust typed closure と同じ wire-level
  fail-closed boundary を提供する。

## Evidence

- RED: `intent_graph_schema_declares_typed_subjects_for_each_consumer` は relation が汎用
  `#/$defs/subject` を参照していたため失敗した。
- GREEN: 同テストで 3 consumer の `$ref` と kind enum を検証した。
- `cargo test -p lsharp-types --test validation_schema`

## Boundary and follow-up

これは JSON Schema と Rust canonical parser の typed subject shape を揃える static contract に限定した
verified partial sliceである。JSON Schema validator の実行、selfhost/native manifest parser、source
producer、current-source stage0 artifact/runtime、Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は未完了で、
TODO の `[~]` を維持する。
