# ADR: v0.3 review attestation の subject/source binding

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: attestation signature、lifecycle、current subject/source/provenance digest の結合
- Related: [`decisions-v0.3-review-lifecycle-verification-gate.md`](decisions-v0.3-review-lifecycle-verification-gate.md)、
  [`decisions-v0.3-review-signature-verification.md`](decisions-v0.3-review-signature-verification.md)

## Context

signature と lifecycle が valid でも、別の canonical graph、source commit、review provenance
へ同じ attestation を再利用できると、review の対象性を証明できない。current snapshot の
identity を caller が渡し、attestation の三つの digest/commit と一致する場合だけ review を
`verified` として扱う必要がある。

## Decision

- `ReviewAttestation::verify_against` は explicit trust store で signature を検証した後、
  `subject_digest`、`source_commit`、`provenance_digest` を caller の current values と byte-for-byte
  比較する。
- いずれかが異なる場合は `stale` とし、別 snapshot の review を `verified` に昇格させない。
- trust key 不明は従来どおり `unverified`、signature 破損は従来どおり verification error とする。
  したがって malformed signature を対象 mismatch に隠さない。
- digest/commit の正規化や network lookup は行わない。current snapshot と provider wire の生成は
  caller/provider adapter の責務とし、検証は offline deterministic に保つ。
- lifecycle gate は同じ呼び出しで適用し、active＋同一 sequence＋三つの identity 一致だけを
  `verified` とする。

## Evidence

- RED: `crates/lsharp-types/tests/review_signature.rs` に matching identity と subject/source/
  provenance 各 mismatch の fixtureを追加した。
- GREEN: 同テスト 8件（署名 3、lifecycle 3、binding 2）が passし、matching identity のみ
  `verified`、mismatch は `stale` になることを固定した。
- Regression: `cargo test -p lsharp-types --lib`（221 passed）、review lifecycle（4 passed）、
  review wire（3 passed）、`cargo clippy -p lsharp-types --all-targets -- -D warnings` を通過した。
- Formatting/contract: changed Rust files の `rustfmt --check`、`git diff --check`、docs audit を通過した。

## Boundary

これは current identity binding の verified partial slice である。expiry clock、manifest review
record との実 projection、CLI/MCP JSON/text report、source/selfhost/native parity、Mac Apple Silicon/
Linux x86_64 artifact/runtime evidence は未完了であり、EC-M3-03〜05 に残す。
