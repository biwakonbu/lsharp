# ADR: v0.2 TypeDef metadata の source projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-syntax/src/parser.rs` と Rust source intent adapter
- Related: `EC-M2-01`, `docs/adr/decisions-v0.2-source-intent-nodes.md`,
  `docs/adr/decisions-v0.2-selfhost-source-adapter.md`

## Context

source intent metadata は `defn` だけでなく型定義にも紐付く。既存 parser は ADT の variant を
閉じ括弧まで読み続けるため、`TypeDef` 後の `:intent` / `:claim` を variant として解釈し、
source graph adapter が扱える `Decl::TypeDef.metadata` を生成できなかった。M2-01 の
source node contract は TypeDef、nested module/private/impl の projection を残件としていた。

## Decision

- ADT `TypeDef` の variant 列は `RParen` または metadata の `Colon` で終端し、既存の
  `try_parse_metadata` へ metadata parsing を委譲する。
- metadata の source order、wire ID、本文、form span は `Decl::TypeDef.metadata` に保持し、
  `validation_source::source_program_to_intent_graph` の既存 TypeDef traversal で typed nodes
  へ投影する。
- RecordDef、selfhost parser、native stage0、artifact/runtime parity はこの slice の範囲外とし、
  M2 aggregate は `[~]` のまま維持する。

## Evidence

- `type_definition_metadata_preserves_source_forms_and_span` は ADT `Result` の variant 後に
  intent/claim forms を置き、宣言順・payload・span を検証する。
- `source_adapter_projects_type_definition_metadata_nodes` は同じ source を typed graph の
  node registry へ投影し、wire ID・本文・source span を検証する。
- `cargo test -p lsharp-syntax --test intent_metadata`、
  `cargo test -p lsharp-types --test validation_source` を通過させる。

## Boundary

これは Rust AST/parser → source adapter の TypeDef metadata verified slice に限定される。
RecordDef metadata、selfhost/native stage0、CLI/MCP、Mac Apple Silicon / Linux x86_64
artifact/runtime、EC-M2-01 aggregate の完了を意味しない。
