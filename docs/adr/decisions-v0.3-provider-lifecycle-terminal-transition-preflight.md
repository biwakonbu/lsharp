# ADR: v0.3 provider review-lifecycle terminal transition preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の sequenced lifecycle transition boundary
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-provider-lifecycle-initial-state-preflight.md`](decisions-v0.3-provider-lifecycle-initial-state-preflight.md)、
  [`decisions-v0.3-provider-lifecycle-terminal-state-preflight.md`](decisions-v0.3-provider-lifecycle-terminal-state-preflight.md)

## Context

native release identity preflight は initial state と terminal-state reactivation を検査していたが、
`proposed` から `revoked` / `superseded` へ直接遷移する eventを受理していた。state allowlistに含まれる値でも、
active reviewを経由しない terminal transitionがprovider input boundaryへ到達できる状態だった。

## Decision

integer `sequence` と string `review_id` を持つ sequenced lifecycle recordでは、直前 stateが `proposed` の場合、
次の stateを `revoked` または `superseded` にしてはならない。該当 eventは
`review lifecycle terminal transition requires active` として fail-closed に拒否する。
既存の initial-state、terminal reactivation、sequence duplicate/rollback、effective_at、state allowlistは維持する。

## Evidence

`test-native-release-identity.py` に `proposed sequence: 1` の後へ `revoked sequence: 2` を置いた fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は previous/current transition境界の診断で拒否する GREENを確認した。
既存 terminal-reactivation fixtureは `proposed → active → revoked → active` に分離した。

## Boundary and follow-up

これは sequenced provider inputの proposed-to-terminal transitionに限る verified partial sliceである。完全な transition matrix、
payload reducer、effective-time ordering、署名/authentication、live provider API取得、MCP semantic parity、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。
blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
