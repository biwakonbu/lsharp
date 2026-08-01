# ADR: provider snapshot の空バイト preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-05` / `prepare-review-evidence-identity.py` / `verify-native-release-identity.py`
- Related: [`decisions-v0.3-provider-snapshot-digest-verification.md`](decisions-v0.3-provider-snapshot-digest-verification.md)

## Context

offline identity producer と verifier は、明示された trust-store / review-lifecycle snapshot が
空でも raw bytes の SHA-256 digest を作成できた。空の provider input は取得済みの snapshot として
扱えず、release identity に provider provenance を付けたまま downstream へ渡せる曖昧な境界だった。

## Decision

provider field の `trust store` と `review lifecycle` に限り、zero-byte inputを fail-closed で拒否する。
producer と verifier の両方で同じ境界を適用し、通常の artifact digest、network、provider API/authentication、
署名・lifecycle意味検証は変更しない。snapshot未指定の旧 unverified compatibility boundaryも維持する。

## Evidence

- RED: 空の trust-store / lifecycle を producer と verifierへ渡す新規 fixtureが成功していた。
- GREEN: 両方が `must be non-empty` で拒否し、producer outputは生成されない。
- producer 6件、release identity 10件、official snapshot/replay-lock/provider-snapshot、stage0 release-package
  focused harnessと shell syntax/diff/docs gateを通過。

## Boundary

これは provider snapshot inputの空値拒否に限る verified partial sliceである。live provider/auth取得、
署名意味検証、current-source Linux runtime、両 target packaged/rollback bytes parityは未検証であり、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9は`[~]`のまま維持する。
