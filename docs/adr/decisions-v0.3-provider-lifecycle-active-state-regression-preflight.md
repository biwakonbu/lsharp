# ADR: v0.3 provider review-lifecycle active state regression preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の sequenced active-state regression boundary
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-provider-lifecycle-terminal-transition-preflight.md`](decisions-v0.3-provider-lifecycle-terminal-transition-preflight.md)、
  [`decisions-v0.3-provider-lifecycle-terminal-state-preflight.md`](decisions-v0.3-provider-lifecycle-terminal-state-preflight.md)

## Context

native release identity preflight は initial state、proposed-to-terminal transition、terminal-state reactivationを検査していたが、
`active` から `proposed` へ戻る sequenced eventを受理していた。state allowlistに含まれる値でも、active reviewを初期状態へ巻き戻す
regressionがprovider input boundaryへ到達できる状態だった。

## Decision

integer `sequence` と string `review_id` を持つ sequenced lifecycle recordでは、直前 stateが `active` の場合、次の stateを
`proposed` にしてはならない。該当 eventは `review lifecycle active state regression` として fail-closed に拒否する。
既存の initial-state、proposed-to-terminal、terminal reactivation、sequence duplicate/rollback、effective_at、state allowlistは維持する。

## Evidence

`test-native-release-identity.py` に `proposed → active → proposed`（sequence `1 → 2 → 3`）fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は previous/current stateを含む regression診断で拒否する GREENを確認した。

## Boundary and follow-up

これは sequenced provider inputの active-to-proposed regressionに限る verified partial sliceである。完全な transition matrix、
payload reducer、effective-time ordering、署名/authentication、live provider API取得、MCP semantic parity、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。
blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
