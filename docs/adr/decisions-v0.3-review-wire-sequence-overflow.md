# ADR: v0.3 review wire sequence の unsigned 64-bit 境界

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `docs/schemas/review-provenance-v1.schema.json` の attestation / lifecycle `sequence`
- Related: `EC-M3-01` / `EC-M3-02`、
  [`decisions-v0.3-review-attestation-sequence-boundary.md`](decisions-v0.3-review-attestation-sequence-boundary.md)

## Context

Rust の review wire model は `sequence: u64` として decode するため、`u64::MAX` を超える JSON integer
は input schema より前に parser が拒否する。一方、schema は `minimum: 1` だけを宣言しており、schema-only
consumer が Rust parser では表現できない超過値を受理できた。sequence の下限だけを揃えても、provider
snapshot の target 間解釈は一致しない。

## Decision

- attestation と lifecycle の `sequence` は JSON Schema で `1 <= sequence <= 18446744073709551615` とする。
- Rust wire parser の `u64` decode boundary を schema に公開し、超過値を schema/parser の双方で fail-closed
  にする。
- sequence の順序、duplicate、rollback、state transition は既存の Rust/selfhost reducer の責務として維持する。

## Evidence

- RED: `wire_rejects_attestation_sequence_overflow` / `wire_rejects_lifecycle_sequence_overflow` は Rust parser
  が超過値を `Schema` error として拒否する一方、schema structural test は `maximum` 欠落で失敗した。
- GREEN: schema に `u64::MAX` の maximum を追加し、両 sequence path の schema boundary を固定した。
- `cargo test -p lsharp-types --test review_attestation --test review_lifecycle --test review_trust_store
  --test review_wire --test validation_schema -- --nocapture`（31 tests）。
- schema JSON、Rust formatting、diff、docs audit は最終 gate で確認する。

## Boundary

これは Rust-host の review wire numeric schema/parser parity に限定した verified partial slice である。
selfhost/native producer、provider snapshot、Mac Apple Silicon / Linux x86_64 runtime、EC-M3 aggregate completion
は残件であり、TODO の `[~]` を維持する。
