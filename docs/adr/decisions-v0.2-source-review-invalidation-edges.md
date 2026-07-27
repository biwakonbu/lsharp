# ADR: v0.2 source evaluates / invalidates typed edges

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-syntax` metadata parser、`crates/lsharp-types` source adapter
- Related: `EC-M2-02` / `EC-M2-03`、`docs/development/planning/v0.2-evidence-graph.md`

## Context

Rust の canonical graph model には ReviewId/ChangeId と
`Edge::Evaluates` / `Edge::Invalidates` が存在する一方、source metadata parser は
`motivates`、`tested-by`、`supports`、`contradicts` までしか ordered form を生成して
いなかった。そのため review が intent/claim/evidence を評価する事実と、change が review/evidence
を無効化する事実を source から graph へ lossless に渡せなかった。

## Decision

- `:evaluates "review:namespace/key" "intent|claim|evidence:namespace/key"` を
  `MetadataFormKind::Evaluates` として保持する。
- `:invalidates "change:namespace/key" "review|evidence:namespace/key"` を
  `MetadataFormKind::Invalidates` として保持する。
- source adapter は ReviewId/ChangeId と subject の kind を strict に復元する。
  Intent/Claim subject は node registry、Evidence subject は evidence registry へ解決し、
  未登録 endpoint、kind mismatch、malformed ID は directive span 付きで fail-closed に拒否する。
- InvalidationSubject::Review は外部 review identity として保持する。review の本文、author、
  provenance、privacy/redaction policy は別の registry/validation slice で扱う。
- 既存 contract inventory ではこれらを executable contract として扱わず、metadata compatibility
  projection の事実として無視する。

## Evidence

- RED: 実装前の
  `cargo test -p lsharp-syntax --test intent_edges review_and_change_edges_preserve_typed_wire_ids_and_source_order -- --nocapture`
  は `MetadataFormKind::Evaluates/Invalidates` 未定義で失敗した。
- GREEN: 同テストを含む `intent_edges` 10件が pass。
- Source adapter: `cargo test -p lsharp-types --test validation_source edges:: -- --nocapture`
  は review/claim/evidence subject、external review invalidation、orphan/mismatch/registry-required
  failure を含む12件が pass。
- CLI integration: `cargo test -p lsharp-driver --test validate_source_review_edges -- --nocapture`
  は `validate --source --format json --emit-manifest` の成功経路で evaluates/invalidates を
  report と manifest へ投影するケース、review subject kind mismatch を manifest 未生成で拒否する
  ケースの2件が pass。
- Regression: `cargo test -p lsharp-types --test metadata_contract -- --nocapture` は4件が pass。

## Boundary

これは Rust-host の source parser → typed graph adapter → `validate --source` report/manifest の
verified slice である。selfhost Parser/IntentSource、native stage0、selfhost manifest/CLI/MCP、
Mac Apple Silicon / Linux x86_64 artifact/runtime parity、review provenance/privacy、EC-M2-02/03
aggregate completion は意味しない。
未接続境界は TODO の `[~]` として維持する。
