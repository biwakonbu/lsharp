# ADR: v0.2 validation report schema optional manifest

- Status: Accepted (verified)
- Date: 2026-07-25
- Scope: `docs/schemas/intent-validation.schema.json`
- Related: `decisions-v0.2-mcp-inline-manifest.md`, `intent-graph.schema.json`

## Context

Rust MCP `lsharp_validate` は `include_manifest: true` のとき、validation report に
canonical version 1 manifest を optional な `manifest` property として返す。一方、公開
report schema は `additionalProperties: false` のままでこの property を宣言していなかった。
schema consumer が実際の MCP structured content を拒否するため、wire contract を同期する。

## Decision

- `intent-validation.schema.json` に optional `manifest` property を追加する。
- manifest の shape は `intent-graph.schema.json` への relative `$ref` で共有する。
- report の既存 required fields は変更せず、manifest は `include_manifest` を指定した場合だけ
  出現する optional field とする。

## Evidence

- RED: `validation_schema::intent_validation_schema_declares_optional_canonical_manifest` が
  manifest property 欠落で失敗。
- GREEN: 同テストを含む schema tests 2件、manifest input tests 10件、clippy、docs audit が pass。

## Boundary

これは公開 report schema と Rust MCP inline output の wire-level parity に限定した sliceである。
schema validator は graph referential closure、duplicate、native/selfhost parity を検証しない。
