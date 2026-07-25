# ADR: v0.2 source evidence enum 診断の span 保持

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_source.rs`
- Related: `EC-M2-02`, `docs/adr/decisions-v0.2-source-evidence-record.md`,
  `docs/adr/decisions-v0.2-source-evidence-boundary.md`

## Context

source `:evidence` record の method、outcome、independence は syntax parser では文字列として
保持され、source adapter が canonical enum へ変換する。変換できない値を拒否する既存の
`InvalidEvidenceField` は field/value だけを持っていたため、CLI や IDE が元の directive を
指し示せず、duplicate/edge error と異なる診断粒度になっていた。

## Decision

- `InvalidEvidenceField` は `field`、`value` に加えて evidence metadata form の `Span` を保持する。
- method、outcome、independence、subject kind の enum/typed conversion failure は同じ
  evidence directive span を返す。
- source parser の field-level span を新たに捏造せず、既存の metadata form span を canonical
  source diagnostic boundary とする。

## Evidence

- `source_adapter_reports_invalid_evidence_enum_with_directive_span` は method/outcome/
  independence/subject の不正値を個別に投影し、field/value とともに evidence directive を含む
  non-empty span を検証する。
- `cargo test -p lsharp-types --test validation_source` は 17 tests pass。
- `cargo test -p lsharp-types` は 209 unit tests と全 integration/doc tests pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、targeted rustfmt、`git diff --check`
  を通過させる。

## Boundary

これは Rust source adapter の enum conversion diagnostic span に限定した verified slice である。
selfhost/native stage0、CLI/MCP report parity、field-level spans、Mac Apple Silicon / Linux
x86_64 artifact/runtime、EC-M2-02 aggregate の完了を意味しない。
