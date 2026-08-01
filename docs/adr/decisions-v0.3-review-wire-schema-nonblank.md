# ADR: v0.3 review wire required field の non-blank schema boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `docs/schemas/review-provenance-v1.schema.json` の required string と optional `reason_digest`
- Related: `EC-M3-01` / `EC-M3-02`、
  [`decisions-v0.2-native-validation-invalid-review-digest.md`](decisions-v0.2-native-validation-invalid-review-digest.md)、
  [`decisions-v0.3-review-wire.md`](decisions-v0.3-review-wire.md)

## Context

Rust の attestation、lifecycle、trust-store model は `trim().is_empty()` を使い、空白だけの required
field を拒否する。selfhost source adapter も同じ non-blank policy を使う。一方、review wire JSON Schema の
`non_empty_string` は `minLength` だけで、`"   "` や NBSP-only の値を schema consumer が受理できた。
optional `reason_digest` も string branch が同じ広い集合を持っていた。

## Decision

- `non_empty_string` に `pattern: "[^\\s]"` を追加し、空白だけの string を拒否する。
- attestation / trust-store の required field は既存の definition を通じて non-blank boundary を共有する。
- lifecycle `reason_digest` は `non_empty_string` または `null` の `anyOf` とし、optional semantics を維持する。
- stable ID、canonical timestamp、base64url、暗号学的検証、provider 取得は既存の別 boundary として維持する。

## Evidence

- RED: `review_provenance_schema_declares_nonblank_required_strings` が definition の pattern 欠落で失敗した。
  Rust wire parser の `subject_digest`、`reason_digest`、trust provider は既に同じ値を fail-closed にする。
- GREEN: Draft 2020-12 validator fixture で attestation subject、lifecycle reason digest、trust provider の
  whitespace-only / NBSP-only 値を schema と Rust parser の双方で拒否する契約を追加した。
- `cargo test -p lsharp-types --test review_attestation --test review_lifecycle --test review_trust_store
  --test review_wire --test validation_schema` と `cargo test -p lsharp-driver --test review_provenance_schema`
  の grouped gate、docs audit、diff check を通過した。

## Boundary

これは Rust-host review wire の required-field schema/parser parity に限定した verified partial slice である。
selfhost/native producer、provider/authentication、Mac Apple Silicon / Linux x86_64 runtime、EC-M3 aggregate
completion は残件であり、TODO の `[~]` を維持する。
