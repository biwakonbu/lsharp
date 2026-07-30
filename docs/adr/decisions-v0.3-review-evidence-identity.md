# ADR: v0.3 review evidence identity の明示 projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp-types` validation report、Rust CLI/MCP review input boundary
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-explicit-context.md`](decisions-v0.3-review-explicit-context.md)、
  [`decisions-v0.3-source-attestation-producer.md`](decisions-v0.3-source-attestation-producer.md)

## Context

review の signature/lifecycle が `verified` になっても、どの source、artifact、trust root、
lifecycle snapshot、明示 clock で判定したかが report に残らなければ、release evidence として
再現できない。反対に current checkout や artifact を暗黙に探索すると、provider/network/Rust
host の状態を report の identity へ混ぜてしまう。

## Decision

- CLI/MCP に `review_artifact_digest` を追加し、`review_subject_digest`、`review_source_commit`、
  `review_now` と同時に渡された場合だけ、`review_evidence_identity` を生成する。artifact を含む
  context は四つの explicit field を all-or-none で要求し、既存の三 field context は後方互換に
  `review_evidence_identity` を生成しない。
- trust store と lifecycle input は parse 後の canonical component JSON
  `{schema_version, trust_store}` / `{schema_version, lifecycle}` を SHA-256 化し、raw public key・
  provider payload を report へ複製しない。入力が省略された component は `null` として明示する。
- Rust `ValidationReport` の optional `review_evidence_identity` は
  `subject_digest`、`source_commit`、`artifact_digest`、`trust_store_digest`、
  `lifecycle_digest`、`now` の順で JSON/text/MCP に投影する。identity は verification state の
  `verified` shortcut ではなく、明示入力の provenance fact とする。
- `review_now` は identity の有無にかかわらず shared strict UTC validator を通す。malformed な
  clock は identity/report へ投影せず、CLI/MCP の input error として止める。
- system clock、environment variable、project 外 path、provider network、manifest からの
  artifact digest 推測は行わない。

## Evidence

- RED: `validation_review_identity`、driver review-input digest/context、CLI、MCP projection/schema
  tests を先に追加し、identity type/API/option が未接続の状態で失敗することを確認した。
- GREEN: `cargo test -p lsharp-types --test validation_review_identity`（2 tests）、driver の
  canonical digest/context unit test、CLI identity JSON/text test、MCP identity/schema test が passした。
- trust-store の JSON field/order を変えても component digest が一致し、lifecycle digest と
  `sha256:` prefix、artifact-bearing context の required field boundary を固定した。
- verification input のない malformed `review_now`、identity model、manifest input の rejection を
  CLI/MCP/types の focused test で追加確認した。

## Boundary and follow-up

これは Rust CLI/MCP report identity の verified partial sliceである。manifest-side identity の
入出力 roundtrip は [`decisions-v0.3-review-evidence-manifest-identity.md`](decisions-v0.3-review-evidence-manifest-identity.md)
で追加した。selfhost/native report parity、native source-file smoke、current-source と packaged
stage0 artifact の provenance、Mac Apple Silicon / Linux x86_64 release/runtime gate は未完了であり、
`TODO.md` の `EC-M3-05` を `[~]` のまま維持する。
