# ADR: v0.3 provider review-lifecycle sequence rollback preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の lifecycle sequence ordering boundary
- Related: [`decisions-v0.3-provider-lifecycle-sequence-duplicate-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-duplicate-preflight.md)、
  [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)

## Context

provider snapshotの state allowlist と duplicate `(review_id, sequence)` 検査だけでは、同じ review の eventが `sequence: 2` から
`sequence: 1` へ戻る入力を許可していた。digestが一致しても append-only lifecycle の順序を壊す eventが release identity boundaryへ
到達できる状態だった。

## Decision

明示 review-lifecycle snapshot の integer sequence recordは、同じ `review_id` の入力順で sequenceが減少してはならない。
直前の sequenceより小さい値は `review lifecycle sequence rollback` として fail-closed に拒否する。既存 duplicate拒否を維持し、
sequence fieldの必須化、effective time ordering、state transition、payload reducerはこのsliceへ追加しない。

## Evidence

`test-native-release-identity.py` に、同じ review の `active sequence: 2` の後へ `revoked sequence: 1` を置いた fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は previous/current sequenceを含む rollback診断で拒否する GREENを確認した。
既存の identity・official release・provider snapshot・stage0 package focused harnessは同じ batchで再実行する。

## Boundary and follow-up

これは sequence rollback の verified partial sliceである。完全な append-only reducer、effective time ordering、state transition、署名/authentication、
live provider API取得、MCP semantic parity、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有の
Lima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
