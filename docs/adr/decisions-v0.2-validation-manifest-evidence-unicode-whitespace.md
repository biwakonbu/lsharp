# ADR: v0.2 validation manifest evidence required fields の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 JSON manifest input
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-validation-input-required-fields.md`

## Context

version 1 manifest の evidence は execution identity と provenance の required string fields を持つ。
source adapter と canonical evidence graph は `str::trim().is_empty()` で ASCII 以外の Unicode White_Space
だけの値も拒否するが、manifest input では `runner` の空文字と coverage bucket の ASCII whitespace
しか回帰テストで固定していなかった。manifest から NBSP-only の provenance が graph に登録できると、
source input と manifest input の fail-closed boundary がずれる。

## Decision

- manifest の evidence required string fields は既存の canonical `EvidenceGraph::add_evidence` validation
  を通し、Unicode White_Space-only の値を `ValidationInputError::Graph(GraphError::InvalidEvidence)`
  として拒否する。
- 対象は execution の `runner` / `target` / `source_commit` / `artifact_digest` と、sampling/provenance
  の `generator` / `producer` / `tool_version` / `timestamp` の8項目とする。
- execution fields を provenance fields より先に検証する既存の field order を保つ。manifest wire schema、
  source diagnostic code/span、selfhost/native CLI surface はこの変更で拡張しない。

## Evidence

- `parse_manifest_rejects_unicode_whitespace_only_required_evidence_fields` を RED として追加し、
  complete manifest の8 required fieldを NBSP-only に変異させ、各 field名を持つ
  `EvidenceValidationError::EmptyField` へ投影することを固定した。
- production code を変更せず、既存の canonical `trim()` policy が manifest input でも適用されることを
  focused test で確認した。
- 実行: `rustfmt --edition 2024 --check crates/lsharp-types/tests/validation_input.rs`
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_unicode_whitespace_only_required_evidence_fields -- --nocapture`

## Boundary and follow-up

これは Rust canonical manifest input の required-field Unicode non-blank policy に限定した verified
partial slice である。manifest の node text/review provenance/coverage Unicode parity、selfhost/native
manifest parser、CLI/MCP report parity、current-source stage0 artifact/runtime、Mac Apple Silicon /
Linux x86_64 artifact matrix、EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
