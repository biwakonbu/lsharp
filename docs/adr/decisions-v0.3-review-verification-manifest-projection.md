# ADR: v0.3 review verification state の manifest projection

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: version 1 intent graph manifest の optional `reviews[].verification_state`
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、
  [`decisions-v0.3-review-verification-report-projection.md`](decisions-v0.3-review-verification-report-projection.md)、
  [`decisions-v0.3-review-explicit-input-boundary.md`](decisions-v0.3-review-explicit-input-boundary.md)

## Context

M2 の `reviews` registry は review ID、opaque provenance digest、visibility だけを持っていた。
attestation の verifier が返す `verified` / `unverified` / `stale` / `revoked` を report だけへ
投影すると、生成した manifest と report の監査事実が一致しない。反対に trust store、signature、
provider payload を manifest へ埋め込むと、既存の privacy boundary と explicit input boundary を壊す。

## Decision

- `ReviewRecord` に optional な `verification_state` を追加し、verifier が明示した state だけを
  registry record の追加 fact として保持する。
- `invalid` は parse/verification error であり、`ReviewRecord` や manifest へ保存できない。
  manifest input の `verification_state: null`、未知値も fail-closed に拒否する。
- manifest output は `reviews[].verification_state` を state 名だけで投影する。state がない
  既存 review は field を省略し、旧 manifest の JSON shape/bytes を変えない。
- `subject_digest`、signature、trust store、lifecycle event はこの manifest projection に複製しない。
  それらは既存の version 1 review provenance wire と explicit CLI/MCP input の責務であり、次の
  verification wiring で state を生成してからこの projection を呼び出す。
- JSON schema と MCP の inline manifest schema は同じ optional state enum を allowlist する。

## Evidence

- RED: `crates/lsharp-types/tests/review_manifest_verification.rs` を先に追加し、未接続の
  `ReviewRecord::with_verification_state` と manifest round-trip contract が compile failure になる
  ことを確認した。
- GREEN: verified state の output/input round-trip、legacy field omission、`invalid`/`null` の
  fail-closed boundary を 5 tests で固定した。
- Regression: `cargo test -p lsharp-types`（221 unit と全 integration）、
  `cargo test -p lsharp-driver --bin lsharp`（180 passed）、両 crate の clippy、schema/MCP tests、
  changed Rust files の rustfmt、intent graph schema の JSON parse を通過した。

## Boundary

これは canonical Rust manifest state projection の verified partial slice である。attestation wire
の manifest-side input、trust/lifecycle/clock から state を生成する CLI/MCP runtime wiring、source/
selfhost/native parity、Mac Apple Silicon/Linux x86_64 artifact/runtime evidence は未完了であり、
EC-M3-03〜05 に残す。
