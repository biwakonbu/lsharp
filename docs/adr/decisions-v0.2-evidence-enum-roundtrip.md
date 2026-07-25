# ADR: v0.2 evidence enum の manifest round-trip

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/{evidence,validation_input,validation_output}.rs`
- Related: `docs/development/planning/v0.2-evidence-graph.md`, `EC-M2-02`

## Context

M2-02 の `Evidence` は method 8 種、outcome 5 種、independence 3 種を canonical model と
manifest wire に持つ。edge variant の round-trip は固定済みだが、既存 fixture は enum の代表値
だけを使っており、serializer と parser の全 variant 対称性を回帰テストで保証していなかった。

## Decision

- manifest serializer は `EvidenceMethod`、`EvidenceOutcome`、`Independence` の全 variant を
  canonical string へ変換する。
- version 1 manifest を parser へ戻した結果は、enum variant を含め元の `IntentGraph` と完全一致
  することを契約とする。
- method/outcome/independence の wire 名は既存の kebab-case policy を維持し、schema version や
  source/native parity の境界は変更しない。

## Evidence

- Contract test: `manifest_output_round_trips_every_evidence_enum_variant` は method 8 種、
  outcome 5 種、independence 3 種を同一 graph へ登録し、JSON serializer → input parser の
  round-trip で graph equality を検証する。
- RED/GREEN: fixture の型推論エラーを修正後、focused test が pass した。production semantics
  の変更はない。

## Boundary

これは Rust `lsharp-types` の Evidence wire coverage に限定した verified slice である。
source syntax adapter、selfhost/native stage0、EmbeddedCli/MCP、両対応 target runtime、
EC-M2 aggregate の完了を意味しない。
