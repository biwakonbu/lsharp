# ADR: v0.3 review verification の explicit identity/clock context

- Status: Accepted (verified partial slice)
- Date: 2026-07-30
- Scope: CLI/MCP から attestation の current identity と expiry clock を渡す境界
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-explicit-state-wiring.md`](decisions-v0.3-review-explicit-state-wiring.md)、
  [`decisions-v0.3-review-attestation-expiry-clock.md`](decisions-v0.3-review-attestation-expiry-clock.md)、
  [`decisions-v0.3-review-attestation-binding.md`](decisions-v0.3-review-attestation-binding.md)

## Context

`ReviewAttestation::verify_against_at` は subject digest、source commit、provenance digest、
明示 UTC clock を受け取れるが、公開 CLI/MCP に context の入口がなかった。system clock や
attestation 自身の identity を current snapshot として使うと、期限切れ・別 graph・別 commit の
review を `verified` と誤認できる。

## Decision

- CLI は `--review-subject-digest`、`--review-source-commit`、`--review-now` を、MCP は同名の
  `review_subject_digest`、`review_source_commit`、`review_now` を受け付ける。
- 三つの field は all-or-none とし、欠落・空値は graph/report/manifest の生成前に診断する。
  `review_now` は caller が渡す canonical UTC timestamp だけを使い、system clock、environment、
  provider/network から補完しない。
- manifest registry の `ReviewRecord.provenance_digest` を review ID ごとの current provenance として
  `verify_against_at` に渡す。registry にない external review は provenance binding を証明できない
  ため `unverified` に留める。
- trust store と lifecycle snapshot が揃う場合だけ identity/clock gate を適用する。trust または
  lifecycle が欠ける場合は、既知 key の signature 破損を診断しつつ、valid signature を暗黙に
  `verified` へ昇格させない。
- `stale`（expiry、subject/source/provenance mismatch）は report/manifest に残し、validation status
  を `unknown` とする。malformed clock/signature は no-report/no-manifest の verification error とする。

## Evidence

- RED: `validate_rejects_partial_review_verification_context_before_report` で、未接続の
  `--review-now` が parser error になり context の all-or-none boundary がなかったことを確認した。
- GREEN: `review_input_cli` 10件で partial context、valid signature、expiry boundary、subject mismatch、
  malformed clock の no-report/no-manifest、report/manifest state projection を固定した。
- MCP schema/behavior: input schema の三つの context field と partial-context error を
  `mcp_server::tests` の validate focused 17件で確認した。
- Regression: `review_signature` 12件、`validation_review_verification` 3件、changed Rust files の
  rustfmt check を通過した。

## Boundary

これは EC-M3-03 の Rust CLI context/expiry/binding verified partial slice である。MCP の valid
signature end-to-end state fixture と malformed `review_now` の no-report/no-manifest contract、
source/selfhost/native producer parity、canonical manifest/artifact digest の自動算出、Mac Apple
Silicon/Linux x86_64 artifact/runtime evidence は未完了であり、EC-M3-03〜05 に残す。
