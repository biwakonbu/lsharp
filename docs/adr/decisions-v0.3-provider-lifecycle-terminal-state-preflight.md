# ADR: v0.3 provider review-lifecycle terminal state preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の sequenced lifecycle terminal-state boundary
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-provider-lifecycle-state-preflight.md`](decisions-v0.3-provider-lifecycle-state-preflight.md)、
  [`decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md)

## Context

native release identity preflight は lifecycle stateのallowlistとsequence orderingを検査していたが、
同じ review の `revoked` / `superseded` 後に `active` eventを置く入力を受理していた。digestが一致していても、
terminal stateを再活性化する eventがprovider input boundaryを通過できる状態だった。

## Decision

integer `sequence` と string `review_id` を持つ sequenced lifecycle recordでは、同じ review の直前 stateが
`superseded` または `revoked` なら後続 eventを `review lifecycle terminal state reactivation` として fail-closed に拒否する。
existing state allowlist、duplicate sequence、sequence rollback、`effective_at` timestamp preflightは維持する。

## Evidence

`test-native-release-identity.py` に `revoked sequence: 1` の後へ `active sequence: 2` を置いた fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は previous/current stateを含む terminal reactivation診断で拒否する GREENを確認した。

## Boundary and follow-up

これは sequenced provider inputの terminal-state reactivationに限る verified partial sliceである。完全な transition matrix、
initial state、payload reducer、effective-time ordering、署名/authentication、live provider API取得、MCP semantic parity、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
