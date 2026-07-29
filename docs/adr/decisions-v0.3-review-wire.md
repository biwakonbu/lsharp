# ADR: v0.3 review provenance/lifecycle wire boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types` version 1 JSON input/output boundary
- Related: `EC-M3-03`、[`review-provenance-v1.schema.json`](../schemas/review-provenance-v1.schema.json)、
  [`decisions-v0.3-review-attestation-canonical-bytes.md`](decisions-v0.3-review-attestation-canonical-bytes.md)、
  [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)

## Context

attestation/lifecycle の canonical model は M2 の source/manifest/CLI にまだ接続されていない。
JSON parser の default map semantics を使うと duplicate key が last-wins になり、provider
snapshot の入力を target ごとに異なる意味へ解釈し得る。未知 field や schema version の暗黙無視も、
署名対象 identity を壊すため許可できない。

## Decision

- version 1 document は `schema_version`、`attestations`、`lifecycle` をすべて必須とする。
- root、attestation、lifecycle の object は custom serde visitor で decode し、unknown field と
  duplicate field を fail-closed に拒否する。
- attestation の signature は padding なし base64url として decode/encode し、non-canonical
  trailing bits、`=`、unknown alphabet、空値を拒否する。
- schema version、algorithm、stable ReviewId、required fields、sequence、lifecycle transition は
  canonical model の error boundary を再利用する。
- JSON output は attestation を `(review_id, sequence)`、lifecycle を registry の
  `(review_id, sequence)` 順で出力し、input order に依存しない。
- JSON schema は [`review-provenance-v1.schema.json`](../schemas/review-provenance-v1.schema.json)
  に固定する。署名検証、trusted key、clock、provider fetch はこの wire parser の責務に含めない。

## Evidence

- RED: `crates/lsharp-types/tests/review_wire.rs` を先に追加し、未公開の wire module import が
  解決できないことを確認した。
- GREEN: `cargo test -p lsharp-types --test review_wire`（3 passed）。
- Boundary tests: unknown/duplicate root・nested fields、unsupported version、unknown algorithm、
  invalid signature encoding、missing lifecycle array、roundtrip bytes を同じ test file で確認した。
- Regression: `review_attestation`（4 passed）、`review_lifecycle`（4 passed）、
  `cargo test -p lsharp-types --lib`（221 passed）。
- Formatting/contract: 新規 Rust files の `rustfmt --check` と `git diff --check` を通過した。

## Boundary

これは Rust canonical wire parser/projection の verified partial slice である。公開 CLI/MCP の
explicit path input、source/selfhost/native parser、manifest schema registration、keyset/lifecycle
snapshot provenance、Mac Apple Silicon/Linux x86_64 runtime evidence は未接続であり、EC-M3-03〜05
の残件として保持する。
