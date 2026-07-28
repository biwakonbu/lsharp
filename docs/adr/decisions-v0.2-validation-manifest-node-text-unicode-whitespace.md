# ADR: v0.2 validation manifest node text の Unicode whitespace boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 JSON manifest node input
- Related: `EC-M2-01`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-node-text-unicode-whitespace.md`

## Context

source adapter と canonical intent node は `str::trim().is_empty()` で空または Unicode White_Space-only
の本文を拒否する。manifest input は JSON string をそのまま `IntentNode` へ渡しているため、source と同じ
node text policy が graph 登録前に適用されることを、NBSP-only fixtureで明示的に固定する必要がある。
ここを受理すると、同じ intent graph が source input と manifest input で異なる意味になる。

## Decision

- version 1 manifest の node `text` は canonical `IntentNode` construction を通し、Unicode White_Space-only
  の値を `ValidationInputError::Node(NodeTextError::EmptyText)` として拒否する。
- stable ID、span、graph reference validation より先に node text の empty boundary を保つ既存順序を変更しない。
- source diagnostic code/span、selfhost/native manifest parser、CLI/MCP surface はこの変更で拡張しない。

## Evidence

- `parse_manifest_rejects_unicode_whitespace_only_node_text` を RED として追加し、complete manifest の
  intent node text を NBSP-only に変異させ、`NodeTextError::EmptyText` へ投影することを固定した。
- production code を変更せず、既存の canonical `trim()` policy が manifest input でも適用されることを
  focused test で確認した。
- 実行: `rustfmt --edition 2024 --check crates/lsharp-types/tests/validation_input.rs`
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_unicode_whitespace_only_node_text -- --nocapture`

## Boundary and follow-up

これは Rust canonical manifest node text の Unicode non-blank policy に限定した verified partial slice
である。manifest の review provenance/coverage Unicode parity、selfhost/native manifest parser、CLI/MCP
report parity、current-source stage0 artifact/runtime、Mac Apple Silicon / Linux x86_64 artifact matrix、
EC-M2-01/EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
