# ADR: v0.3 review wire trust-store duplicate boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `docs/schemas/review-provenance-v1.schema.json` の `trust_store` array と Rust wire parser
- Related: `EC-M3-01`、[`decisions-v0.3-review-wire.md`](decisions-v0.3-review-wire.md)

## Context

`ReviewTrustStore` は `(provider, key_id, algorithm)` が同じ key entry を duplicate として拒否する。
しかし JSON Schema の `trust_store` array に `uniqueItems` がなく、同一 object を重複させた入力を
schema consumer が受理できた。この入力境界の差は、consumer ごとに trust snapshot の解釈が変わる
余地を残す。

## Decision

- `trust_store` に `uniqueItems: true` を設定し、同一 key entry object の重複を schema でも拒否する。
- Rust custom visitor / `ReviewTrustStore::add_key` の duplicate identity 検査は維持する。
- `provider`・`key_id`・`algorithm` が同じで公開鍵だけ異なる semantic duplicate は JSON Schema の
  標準語彙だけでは一意性を表現できないため、Rust canonical parser の fail-closed boundary として残す。

## Evidence

- RED: `review_provenance_schema_declares_unique_trust_store_entries` が、schema に `uniqueItems` がなく
  失敗することを確認した。
- GREEN: schema を更新し、`lsharp-types` の validation schema 11件と trust-store 3件を通過した。
- driver contract `review_wire_schema_rejects_duplicate_trust_store_entries` で、schema validator と
  Rust parser が同じ exact duplicate を拒否することを確認する。

## Boundary

これは exact duplicate object の schema/parser parity に限定した verified partial slice である。
semantic composite duplicate、provider snapshot 取得、署名検証、selfhost/native producer、Mac/Linux
artifact/runtime、EC-M3 aggregate completion は残件である。
