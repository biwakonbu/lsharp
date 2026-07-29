# ADR: v0.3 review verification state の report projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `IntentGraph` の明示 verification fact と Rust validation report JSON/text
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-attestation-expiry-clock.md`](decisions-v0.3-review-attestation-expiry-clock.md)

## Context

attestation model は signature、subject/source/provenance、lifecycle、expiry clock を検証できる
ようになったが、`IntentGraph::validate()` は M2 の opaque review evidence だけを数えていた。
このままでは `stale` や `revoked` を独立 review gate から除外する判断と、JSON/text の監査事実が
report に残らない。既存の M2 manifest/report bytes を暗黙に変えず、次の CLI/MCP wiring が同じ
canonical projection を利用できる境界が必要だった。

## Decision

- `ReviewVerificationFact` は `(ReviewId, ReviewVerificationState)` の一つの graph fact として
  明示的に受け取る。`Invalid` は report へ投影せず `ReviewVerificationProjectionError` で
  fail-closed に拒否する。
- `IntentGraph::validate_with_review_verifications` は facts を review ID の wire 順に並べ、
  duplicate ID を拒否する。既存 `validate()` は state を暗黙に補わず、M2 の出力互換を保つ。
- verification facts が渡された場合、`independent_reviews` は `verified` review が
  `evaluates` する pass/independent evidence だけを数える。`unverified`、`stale`、`revoked`
  は gate を満たさず report status を `unknown` にする。contradiction の `fail` precedence は
  既存 report policy を維持する。
- JSON は optional `review_verifications` 配列（`review_id`、`state`）を、text は同じ順序の
  `review-verification: <id>=<state>` 行を追加する。facts がない従来 report では field/行を省略する。
- `intent-validation.schema.json` と MCP output schema は optional facts を allowlist するが、
  この slice では CLI/MCP が trust/lifecycle input から facts を暗黙生成しない。

## Evidence

- RED: `crates/lsharp-types/tests/validation_review_verification.rs` を先に追加し、未公開の
  fact/API、verified gate、non-verified unknown、sort/duplicate/invalid boundary が未接続で
  compile failure になることを確認した。
- GREEN: 同テスト 3件が passし、verified の complete graph は `pass`/`independent_reviews=1`、
  non-verified は `unknown`/`independent_reviews=0`、JSON/text の deterministic projection、
  duplicate/invalid fail-closed を固定した。
- Regression: `cargo test -p lsharp-types --lib`（221 passed）、validation focused tests、
  `cargo clippy -p lsharp-types --all-targets -- -D warnings`、changed Rust files の rustfmt、
  validation schema/MCP schema の JSON contract を通過した。

## Boundary

これは canonical Rust report projection の verified partial slice である。version 1 manifest の
optional `reviews[].verification_state` projection は別 ADR で追加済みだが、attestation/lifecycle
input projection、CLI の explicit wire wiring、MCP runtime facts、source/selfhost/native parity、
Mac Apple Silicon/Linux x86_64 artifact/runtime evidence は未完了であり、EC-M3-01〜05 に残す。
