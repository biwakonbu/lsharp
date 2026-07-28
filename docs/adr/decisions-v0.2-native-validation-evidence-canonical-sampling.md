# ADR: v0.2 canonical evidence sampling coverage validation

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/evidence.rs`, `crates/lsharp-types/tests/evidence_required_fields.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-bucket.md`

## Context

前の source adapter 修正で source `:evidence` の空 coverage bucket は directive span 付きで拒否
されるようになったが、canonical `SamplingPlan::new` を直接使う consumer では
`EvidenceGraph::add_evidence` が sampling coverage を検査せず、同じ `{"": count}` を登録できた。
graph の入口ごとに deterministic sampling の fail-closed policy が分かれる状態だった。

## Decision

- `SamplingPlan::validate_required_fields` は required generator に加えて、coverage map の空文字
  bucket を `EvidenceValidationError::EmptyField { field: "coverage" }` として拒否する。
- `Evidence::validate_required_fields` から `EvidenceGraph::add_evidence` まで既存の validation
  chain を利用し、invalid sampling は登録前に `GraphError::InvalidEvidence` へ投影する。
- source adapter の directive span、selfhost の stable code/span、canonical model の field error
  はそれぞれの境界に必要な情報を保持し、duplicate/whitespace policy は別 slice として残す。

## Evidence

- RED: `sampling_rejects_empty_coverage_bucket_before_graph_registration` は修正前に
  `SamplingPlan::validate_required_fields` が成功した。
- GREEN: `cargo test -p lsharp-types --test evidence_required_fields -- --nocapture`（6 passed）と
  `cargo test -p lsharp-types --test evidence_graph -- --nocapture`（5 passed）。
- Regression: `cargo test -p lsharp-types --test validation_source -- --nocapture`（54 passed）。
- Native contract: Linux x86_64 native stage0 source-file smoke と provenance gate が通過した。

## Boundary and follow-up

これは canonical model の empty coverage bucket validation に限定した verified partial sliceである。
coverage の duplicate/whitespace policy、count と cases の意味論、manifest/validate CLI、selfhost
current-source artifact/runtime、Mac/Linux matrix、EC-M2-02 aggregate は未完了であり、TODO の `[~]`
を維持する。
