# ADR: v0.3 review wire lifecycle timestamp schema parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `docs/schemas/review-provenance-v1.schema.json` と Rust review wire の lifecycle timestamp boundary
- Related: `EC-M3-01` / `EC-M3-02`、[`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)

## Context

`ReviewLifecycleEvent::new` は `effective_at` を attestation と同じ strict canonical UTC timestamp
として検証し、形式不正・存在しない日付・秒範囲外を拒否する。一方、review provenance wire schema の
`lifecycle.effective_at` は `non_empty_string` だけを参照していたため、schema consumer は
`"tomorrow"` や offset 付き時刻を有効と判定でき、Rust parser より広い入力境界になっていた。

## Decision

- `lifecycle.effective_at` は `#/$defs/canonical_utc_timestamp` を参照する。
- attestation の `issued_at` / `expires_at` と同じ `YYYY-MM-DDTHH:MM:SSZ` の lexical boundary を wire
  schema で要求する。
- 実在する暦日、秒範囲、lifecycle の順序・遷移は Rust canonical parser/reducer の責務として維持する。
- 他の lifecycle field、後方互換の optional `trust_store`、既存の version 1 envelope は変更しない。

## Evidence

- RED: `review_provenance_schema_requires_canonical_timestamp_for_lifecycle_effective_at` を追加し、
  `non_empty_string` 参照の schema が失敗することを確認した。
- GREEN: schema の参照を canonical timestamp に変更し、`cargo test -p lsharp-types --test validation_schema`
  （8 passed）と `cargo test -p lsharp-types --test review_wire`（5 passed）を通過した。
- 既存の `wire_rejects_noncanonical_lifecycle_effective_timestamp` は `2026-02-30T00:00:00Z` を Rust
  parser が拒否する runtime boundary を維持する。schema JSON の構文と差分検査も通過した。

## Boundary

これは Rust canonical wire schema と parser の lexical timestamp parity に限定した verified partial slice
である。schema consumer の実 validator matrix、provider snapshot、signature/authentication、
selfhost/native producer、Mac Apple Silicon / Linux x86_64 の artifact/runtime evidence、EC-M3 aggregate
completion は残件である。
