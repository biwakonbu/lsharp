# ADR: v0.3 review attestation の lifecycle verification gate

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: Rust canonical attestation signature と explicit lifecycle snapshot の結合
- Related: [`decisions-v0.3-review-signature-verification.md`](decisions-v0.3-review-signature-verification.md)、
  [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)

## Context

trusted key に対する署名が正しくても、review が現在も有効であることや、別 sequence の
attestation ではないことは証明できない。lifecycle を省略して `verified` へ昇格させると、
失効・差し替え済み review が独立 review gate に残る。

## Decision

- `ReviewAttestation::verify_with_lifecycle` は先に explicit trust store と canonical bytes の
  signature を検証し、signature が verified でない場合はその状態を保持する。
- lifecycle event がない、または現在 state が `proposed` の場合は `unverified` とする。
- 現在 state が `active` かつ lifecycle sequence と attestation sequence が一致した場合だけ
  `verified` とする。
- `active` の sequence 不一致は対象 snapshot の差し替えとして `stale` とする。
- `superseded` は `stale`、`revoked` は `revoked` とする。terminal state は sequence 不一致より
  優先し、失効・差し替えの事実を隠さない。
- lifecycle snapshot の malformed wire/遷移は既存 parser error、signature の破損は既存
  `AttestationVerificationError` として fail-closed に返す。

## Evidence

- RED: `crates/lsharp-types/tests/review_signature.rs` に active/missing/proposed/superseded/
  revoked/sequence mismatch の同一署名 fixtureを追加した。
- GREEN: 同テスト 6件が passし、active+same sequence だけが `verified`、他の状態が
  `unverified`/`stale`/`revoked` へ分離されることを固定した。
- Regression: `cargo test -p lsharp-types --lib`（221 passed）、新規 Rust files の rustfmt check、
  `git diff --check` を通過した。

## Boundary

これは canonical lifecycle gate の verified partial slice である。subject/source/provenance digest
binding は別 ADR の canonical partial slice として追加済みだが、expiry clock、manifest projection、
CLI/MCP report の state projection、selfhost/native parity、Mac Apple Silicon/Linux x86_64
artifact/runtime evidence は未完了であり、EC-M3-03〜05 に残す。
