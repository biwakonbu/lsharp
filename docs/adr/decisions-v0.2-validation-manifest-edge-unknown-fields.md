# ADR: v0.2 validation manifest edge payload の未知 field 拒否

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input/manifest.rs` の version 1 JSON `edges`
- Related: `EC-M2-02`、`EC-M2-03`、`EC-M3-01`

## Context

version 1 manifest の node、evidence、review、sampling などの struct payload は
`deny_unknown_fields` で入力 schema を閉じていた。一方、`relation` を tag に持つ
`EdgeInput` は serde の internally tagged enum の通常 derive に依存しており、variant
payload に追加された未知 field を黙って無視していた。これは同じ bytes が異なる graph
へ解釈され得るため、canonical manifest parity の fail-closed 境界を弱める。

## Decision

- edge の JSON object は custom deserializer で relation ごとの許可 field を先に検査する。
- `motivates`、`constrained-by`、`tested-by`、`supports`、`contradicts`、`evaluates`、
  `invalidates` それぞれの payload に relation と定義済み endpoint だけを許可する。
- 未知 field、duplicate field、欠落した relation は `ValidationInputError::Json` として
  graph 登録前に fail-closed にする。
- edge 以外の schema、source adapter、CLI/MCP、selfhost/native runtime の責務は拡張しない。

## Evidence

- RED: `parse_manifest_rejects_unknown_fields_in_edge_payloads` は実装前に未知 field を
  含む edge が受理されることを確認して失敗した。
- GREEN: 全 6 edge relation variant の未知 field fixture が `ValidationInputError::Json`
  で拒否されることを固定した。
- `cargo test -p lsharp-types --test validation_input`
- `cargo test -p lsharp-types`
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`
- 変更対象ファイルの `rustfmt --edition 2024 --check` と `git diff --check`
- 公開 `lsharp validate` の `validate_rejects_unknown_edge_field_without_report_or_manifest_output`
  で exit `1`、空 stdout、manifest file なし、unknown field/edge 診断を固定した。
- `cargo test -p lsharp-driver --test manifest_input_cli`（2 tests）
- `cargo clippy -p lsharp-driver --test manifest_input_cli -- -D warnings`

## Boundary and follow-up

これは Rust canonical manifest input と公開 Rust CLI の edge schema boundary に限定した verified partial
slice である。selfhost/native manifest parser、source producer、MCP、atomic/durable writer、
current-source stage0 artifact/runtime、Mac Apple Silicon / Linux x86_64 matrix、
EC-M2-02 / EC-M2-03 / EC-M3 aggregate は未完了であり、TODO の `[~]` を維持する。
