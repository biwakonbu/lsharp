# ADR: v0.3 review の explicit trust store

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types` の trusted public key registry と review wire optional field
- Related: `EC-M3-03`、[`review-provenance-v1.schema.json`](../schemas/review-provenance-v1.schema.json)、
  [`semantic-contract-system.md`](../language/semantic-contract-system.md)、
  [`decisions-v0.3-review-wire.md`](decisions-v0.3-review-wire.md)

## Context

semantic contract system は human review の trust root を current change の中へ置かず、
external trust store または signed baseline から供給する契約を持つ。v0.3 review wire が
attestation だけを受け付けると、caller がどの鍵を信頼したかを deterministic に渡せず、
network/environment の暗黙取得や manifest 自身による self-approval へ流れる余地が残る。

## Decision

- `ReviewTrustKey` は provider、key ID、algorithm、32-byte Ed25519 public key と active state を持つ。
- `ReviewTrustStore` は `(provider, key_id, algorithm)` を一意キーとした deterministic registry とする。
- 空 identity、key length 不一致、duplicate key は fail-closed に拒否する。
- review wire の `trust_store` は optional。省略した document は後方互換に parse するが、
  caller が明示した trust store なしに `verified` を推論してはならない。
- wire の trust store は allowlist input の projectionであり、signature verification、provider fetch、
  trust root の署名検証は後続 boundaryとする。key rotationの active selectionは
  [`decisions-v0.3-review-trust-store-active-key-rotation.md`](decisions-v0.3-review-trust-store-active-key-rotation.md)
  で定義する。
- CLI の公開 flag 名は既存 semantic contract system と同じ `--trust-store` を採用する。wire field
  は `trust_store` とし、`review_keyset` という別名を暗黙に増やさない。

## Evidence

- RED: `crates/lsharp-types/tests/review_trust_store.rs` を先に追加し、trust-store module と
  wire getter が未実装であることを確認した。
- GREEN: `cargo test -p lsharp-types --test review_trust_store`（3 passed）。
- Regression: wire（3 passed）、attestation（4 passed）、lifecycle（4 passed）、
  `cargo test -p lsharp-types --lib`（221 passed）。
- Schema: `python3 -m json.tool docs/schemas/review-provenance-v1.schema.json` を通過し、
  `trust_store`／`trust_key` の unknown field/Ed25519 key shape を schema に反映した。
- Formatting/contract: 新規 Rust files の `rustfmt --check` と `git diff --check` を通過した。

## Boundary

これは explicit trust-store input と canonical JSON projection の verified partial slice である。
署名検証、trusted baseline replacement attack、CLI `--trust-store` path boundary、MCP/source/
selfhost/native parity、Mac Apple Silicon/Linux x86_64 runtime evidence は未完了であり、
EC-M3-03〜05 の残件として保持する。
