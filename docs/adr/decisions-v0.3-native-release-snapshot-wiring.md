# ADR: v0.3 native-only release の provider snapshot wiring

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `scripts/release.sh` の native-only App.Cli release identity gate
- Related: [`decisions-v0.3-provider-snapshot-digest-verification.md`](decisions-v0.3-provider-snapshot-digest-verification.md)、
  [`decisions-v0.3-release-identity-gate.md`](decisions-v0.3-release-identity-gate.md)、
  [`v0.3-milestone-01.md`](../development/planning/v0.3-milestone-01.md)

## Context

verifier は明示された trust-store/lifecycle snapshot を再計算できるようになったが、native-only
`release.sh` が snapshot path を渡さなければ、release gate は identity digest の形状だけを検証する
状態に戻ってしまう。release caller が snapshot を用意した場合に、その検証機能を実際の artifact
packaging 境界へ接続する必要がある。

## Decision

- native-only release は `NATIVE_ONLY_REVIEW_TRUST_STORE` と
  `NATIVE_ONLY_REVIEW_LIFECYCLE` を任意の明示 path として受け取る。
- 片方だけ、空/存在しない snapshot、または identity file なしの snapshot 指定は fail-closed に拒否する。
- 両方が指定された場合は `verify-native-release-identity.py` へそのまま渡し、raw bytes と identity
  digest の一致を release packaging 前に検証する。
- snapshot bytes は公開 archive へコピーせず、provider/caller の入力境界に留める。snapshot 未指定の
 既存 release 呼び出しは従来の explicit digest boundary として維持する。

## Evidence

RED は、identity と改ざん済み trust-store snapshot を渡した native-only release が snapshot path
を無視して成功する focused test の失敗。GREEN は正しい snapshot の release packaging、trust-store
digest mismatch の non-zero、preparer/verifier の既存 roundtrip を含む Python suite で確認した。

## Boundary

この slice は native-only App.Cli の `scripts/release.sh` wiring に限定される。stage0 package、release
smoke、native-official multi-target orchestrator、provider API/authentication、Mac/Linux runtime の
snapshot propagation は次の gateで閉じる。`EC-M3-05` は `[~]` のまま残す。
