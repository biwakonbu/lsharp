# ADR: v0.3 review evidence identity の manifest projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp-types` intent graph manifest input/output、Rust CLI `--emit-manifest`、MCP inline manifest
- Related: [`decisions-v0.3-review-evidence-identity.md`](decisions-v0.3-review-evidence-identity.md)、
  [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)

## Context

`review_evidence_identity` を validation report だけへ投影すると、`--emit-manifest` と MCP の
inline manifest に release evidence の source/artifact/trust/lifecycle identity が残らない。
manifest を後から再検証したとき、report と成果物の provenance を同じ canonical object として
roundtrip できる必要がある。

## Decision

- intent graph manifest に optional な top-level `review_evidence_identity` object を追加する。
  field order は `subject_digest`、`source_commit`、`artifact_digest`、`trust_store_digest`、
  `lifecycle_digest`、`now` とし、trust/lifecycle の欠落は `null` を明示する。
- manifest input は object の全 field と `null` を要求し、unknown field、空 required field、欠落 field
  を fail-closed に拒否する。identity は graph、validation report、manifest output の同じ fact として
  roundtrip する。
- CLI/MCP の explicit context から新しい identity を投影するとき、manifest に既存 identity があれば
  byte-equivalent な値だけを許可し、異なる値で上書きしない。source、artifact、clock、trust root を
  current checkout、environment、network から推測しない。
- MCP の inline manifest schema と repository schema は同じ optional object shape を宣言する。

## Evidence

- RED: manifest projection/roundtrip、schema、CLI `--emit-manifest`、MCP inline manifest の tests を
  identity 接続前に追加し、型/API不足または manifest field 欠落で失敗することを確認した。
- GREEN: `cargo test -p lsharp-types --test validation_manifest_review_identity`（3 tests）、
  `cargo test -p lsharp-types --quiet`、driver bin（189 tests）、review-input CLI（12 tests）、
  MCP identity/schema tests が passした。
- manifest identity の canonical JSON field order、nullable digest の明示、input roundtrip、既存値との
  conflict rejection、CLI/MCP output projection を同じ fixture で固定した。

## Boundary and follow-up

これは Rust canonical manifest/report と Rust CLI/MCP の verified partial sliceである。selfhost/native
producer parity、current-source と packaged stage0 artifact の provenance、provider helper boundary、
Mac Apple Silicon / Linux x86_64 の release artifact/runtime gate は未完了であり、`TODO.md` の
`EC-M3-05` は `[~]` のまま維持する。
