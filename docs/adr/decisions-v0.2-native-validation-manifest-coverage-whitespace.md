# ADR: v0.2 validation manifest coverage whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 manifest input と canonical evidence validation
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-whitespace.md`

## Context

source adapter と canonical `SamplingPlan` では whitespace-only coverage bucket を拒否していたが、
version 1 JSON manifest の `parse_intent_graph_json` 入口には回帰テストがなかった。manifest parser が
canonical graph 登録へ同じ required-field policy を渡すことを明示的に固定する必要がある。

## Decision

- `parse_intent_graph_json` は whitespace-only coverage bucket を `GraphError::InvalidEvidence` の
  `EvidenceValidationError::EmptyField { field: "coverage" }` として graph 登録前に拒否する。
- manifest input では source directive span を持たないため、canonical field error の shape を保持し、
  source-specific value/span 診断は source adapter の境界に残す。
- manifest の native stage0/runtime parity は別の EC-M3-01/02 artifact contract として扱う。

## Evidence

- `parse_manifest_rejects_whitespace_only_coverage_bucket_before_registration` を追加し、
  `cargo test -p lsharp-types --test validation_input -- --nocapture`（17 passed）を確認した。
- `rustfmt --edition 2024 --check crates/lsharp-types/tests/validation_input.rs` と
  `git diff --check` が通過した。

## Boundary and follow-up

これは Rust canonical manifest input の whitespace policy 回帰に限定した verified partial sliceである。
manifest の native source/runtime parity、JSON/text report、atomic writer、coverage count/cases、
Unicode whitespace、Mac/Linux matrix、EC-M2-03 aggregate は未完了であり、TODO の `[~]` を維持する。
