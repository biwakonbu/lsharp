# ADR: v0.2 RecordDef metadata の source projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: Rust syntax AST/parser と source intent adapter
- Related: `EC-M2-01`, `docs/adr/decisions-v0.2-source-type-definition-metadata.md`,
  `docs/adr/decisions-v0.2-source-intent-nodes.md`

## Context

ADT の `TypeDef` は metadata を保持して source graph へ投影できる一方、record 型の
`RecordDef` は record 本体の閉じ括弧直後をすぐに `type` の閉じ括弧として消費していた。
そのため record に `:intent` / `:claim` などを付けると parse error になり、型定義間で
source node contract が不統一だった。

## Decision

- `Decl::RecordDef` に `metadata: Option<Metadata>` を追加し、record field list の後で
  既存の shared `try_parse_metadata` を呼び出す。
- `validation_source::source_program_to_intent_graph` は `Defn` / `TypeDef` と同じ順序で
  `RecordDef` の node、evidence、edge metadata を投影する。
- wire ID、本文、source order、directive span は既存 metadata contract をそのまま使い、
  record 名や field schemaから ID を推測しない。

## Evidence

- `record_definition_metadata_preserves_source_forms_and_span` は parametric record の
  metadata forms、wire ID、本文、source span を parser AST で検証する。
- `source_adapter_projects_record_definition_metadata_nodes` は同じ source metadata を
  typed `IntentGraph` node registry へ投影する。
- `source_adapter_projects_record_definition_evidence_and_support_edges` は RecordDef 上の
  evidence registry と `supports` edge も node collection 後に投影することを検証する。
- `validate_source_projects_record_definition_metadata_into_report_and_manifest` は同じ
  RecordDef source を公開 Rust `validate --source --emit-manifest` へ通し、report の status と
  version 1 manifest の node/evidence/edge wire fields を検証する。
- `cargo check --workspace`
- `cargo test -p lsharp-syntax --test intent_metadata`
- `cargo test -p lsharp-types --test validation_source`
- `cargo test -p lsharp-driver --test validate_cli validate_source_projects_record_definition_metadata_into_report_and_manifest`

## Boundary

この slice は Rust AST/parser → Rust source adapter の RecordDef metadata に限定する。
selfhost parser/native stage0 parity、RecordDef の CLI/MCP manifest emission、evidence/edge の
aggregate、Mac Apple Silicon / Linux x86_64 artifact/runtime parity、EC-M2-01 aggregate の完了は
意味しない。
