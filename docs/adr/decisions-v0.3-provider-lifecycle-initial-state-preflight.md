# ADR: v0.3 provider review-lifecycle initial state preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の first sequenced lifecycle state boundary
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-provider-lifecycle-terminal-state-preflight.md`](decisions-v0.3-provider-lifecycle-terminal-state-preflight.md)、
  [`decisions-v0.3-provider-lifecycle-state-preflight.md`](decisions-v0.3-provider-lifecycle-state-preflight.md)

## Context

native release identity preflight は lifecycle state allowlist、sequence ordering、terminal-state reactivationを検査していたが、
sequenced reviewの最初の eventを `revoked` または `superseded` にしても受理していた。allowlistに含まれる値でも、review lifecycleの
初期状態として不正な eventがprovider input boundaryへ到達できる状態だった。

## Decision

integer `sequence` と string `review_id` を持つ同一 reviewの最初の sequenced recordは `proposed` または `active` に限定する。
最初の stateがそれ以外なら `review lifecycle initial state must be one of active, proposed` として fail-closed に拒否する。
sequenceなし既存 snapshotの互換性、既存 state allowlist・sequence duplicate/rollback・effective_at・terminal-state preflightは維持する。

## Evidence

`test-native-release-identity.py` に `revoked sequence: 1` だけを持つ lifecycle fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は allowed initial statesを含む診断で拒否する GREENを確認した。
terminal-state reactivation fixtureは `proposed → revoked → active` に分離し、failure boundaryの独立性も確認した。

## Boundary and follow-up

これは sequenced provider inputの initial-state ruleに限る verified partial sliceである。完全な transition matrix、
state transition payload、effective-time ordering、署名/authentication、live provider API取得、MCP semantic parity、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
