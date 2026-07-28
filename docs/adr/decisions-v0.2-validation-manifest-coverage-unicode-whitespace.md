# ADR: v0.2 validation manifest coverage bucket の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 JSON manifest sampling.coverage
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-evidence-coverage-unicode-whitespace.md`

## Context

source adapter、canonical `SamplingPlan`、selfhost Evidence consumer は coverage bucket 名を
`str::trim().is_empty()` 相当の non-blank policyで検証する。manifest input には ASCII whitespace-only
bucket の回帰があったが、NBSP-only bucket が graph 登録前に同じ `EmptyField` へ投影されることを
固定していなかった。wire inputだけが別 policyになると、同じ evidence が source と manifest で変質する。

## Decision

- manifest の `sampling.coverage` は canonical `SamplingPlan` validation を通し、Unicode White_Space-only
  の bucket key を `ValidationInputError::Graph(GraphError::InvalidEvidence)` / field `coverage` として
  graph 登録前に拒否する。
- 元の coverage bucket valueを後段で trim して書き換えず、duplicate/count/cases の意味論は別の境界として
  維持する。
- source diagnostic code/span、selfhost/native manifest parser、CLI/MCP surface はこの変更で拡張しない。

## Evidence

- `parse_manifest_rejects_unicode_whitespace_only_coverage_bucket_before_registration` を RED として追加し、
  complete manifest の coverage key を NBSP-only に変異させ、`EmptyField { field: "coverage" }` へ
  投影することを固定した。
- JSON fixtureの Unicode escape を実際の NBSP code pointへ修正した後、production codeを変更せず
  canonical validationが manifest inputでも適用されることを focused test で確認した。
- 実行: `rustfmt --edition 2024 --check crates/lsharp-types/tests/validation_input.rs`
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_unicode_whitespace_only_coverage_bucket_before_registration -- --nocapture`

## Boundary and follow-up

これは Rust canonical manifest coverage bucket の Unicode non-blank policy に限定した verified partial
slice である。coverage count/cases の意味論、manifest の他 field parity、selfhost/native manifest parser、
CLI/MCP report parity、current-source stage0 artifact/runtime、Mac Apple Silicon / Linux x86_64 artifact
matrix、EC-M2-02/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
