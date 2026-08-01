# ADR: v0.3 provider review-lifecycle proposed self-transition preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の sequenced proposed-state transition boundary
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-provider-lifecycle-active-state-self-transition-preflight.md`](decisions-v0.3-provider-lifecycle-active-state-self-transition-preflight.md)、
  [`decisions-v0.3-provider-lifecycle-initial-state-preflight.md`](decisions-v0.3-provider-lifecycle-initial-state-preflight.md)

## Context

native release identity preflight は initial state、proposed-to-terminal transition、active-state transitionsを検査していたが、sequenceが増加しても
`proposed → proposed` の no-op eventを受理していた。state allowlistに含まれる値でも、review lifecycleを進めない proposed self-transitionがprovider input boundaryへ到達できる状態だった。

## Decision

integer `sequence` と string `review_id` を持つ sequenced lifecycle recordでは、直前 stateが `proposed` の場合、次の stateも `proposed` にしてはならない。
該当 eventは `review lifecycle proposed state self-transition` として fail-closed に拒否する。既存の initial-state、active self-transition/regression、proposed-to-terminal、terminal reactivation、
sequence duplicate/rollback、effective_at、state allowlistは維持する。

## Evidence

`test-native-release-identity.py` に `proposed → proposed`（sequence `1 → 2`）fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は proposed self-transition診断で拒否する GREENを確認した。

## Boundary and follow-up

これは sequenced provider inputの proposed self-transitionに限る verified partial sliceである。完全な transition matrix、payload reducer、effective-time ordering、署名/authentication、
live provider API取得、MCP semantic parity、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processもあるため
Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
