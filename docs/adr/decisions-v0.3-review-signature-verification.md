# ADR: v0.3 review attestation の Ed25519 verification

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types` canonical attestation model と explicit trust store の signature verification
- Related: `EC-M3-03` / `EC-M3-04`、[`decisions-v0.3-review-attestation-canonical-bytes.md`](decisions-v0.3-review-attestation-canonical-bytes.md)、
  [`decisions-v0.3-review-trust-store.md`](decisions-v0.3-review-trust-store.md)

## Context

attestation の signature bytes を保持するだけでは、review の独立性を判定できない。
一方、manifest/current change に同梱された public key をそのまま信頼すると、同じ変更が
自分自身を human review として承認できる。semantic contract system の trust-root policy に
合わせ、caller が明示的に渡した trust store と canonical bytes だけで offline verification を行う。

## Decision

- Ed25519 verification は `ed25519-dalek` を使い、Rust canonical model に閉じ込める。
- attestation の `(provider, key_id, algorithm)` と trust store の同じ identity を lookup する。
- key が存在しない場合は暗黙に成功させず `ReviewVerificationState::Unverified` を返す。
- key が存在する場合、signature length、public key、canonical bytes に対する署名を検証する。
  length/encoding/mismatch は `AttestationVerificationError` として fail-closed に返す。
- 検証対象 bytes は `ReviewAttestation::canonical_bytes()` のみとし、provider JSON、署名自身、
  network response の順序には依存しない。
- 期限、subject/source digest の current snapshot 一致はこの sliceでは判定せず、後続の
  lifecycle/report boundaryへ残す。lifecycle state と attestation sequence の canonical gate は
  [`decisions-v0.3-review-lifecycle-verification-gate.md`](decisions-v0.3-review-lifecycle-verification-gate.md)
  で別の fact として接続する。

## Evidence

- RED: `crates/lsharp-types/tests/review_signature.rs` を先に追加し、verification API と
  `ed25519-dalek` dependency が未接続であることを確認した。
- GREEN: `cargo test -p lsharp-types --test review_signature`（3 passed）。
  deterministic signing key、trusted key verification、missing-key `unverified`、tampered/
  malformed signature fail-closed を確認した。
- Regression: trust store（3 passed）、wire（3 passed）、attestation（4 passed）、
  lifecycle（4 passed）、`cargo test -p lsharp-types --lib`（221 passed）。
- Formatting/contract: 新規 Rust files の `rustfmt --check` と `git diff --check` を通過した。

## Boundary

これは Rust canonical signature verification の verified partial slice である。CLI/MCP の
`--trust-store` path/root replacement protection と lifecycle state gate は別 ADR で部分実装済み。
manifest projection、source/selfhost/native parity、Mac Apple Silicon/Linux x86_64 artifact/runtime
evidence は未完了であり、subject/source binding と strict expiry clock は別 ADR の canonical
partial slice として追加済みである。EC-M3-03〜05 の残件として扱う。
