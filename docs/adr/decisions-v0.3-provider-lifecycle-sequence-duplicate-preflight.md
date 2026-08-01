# ADR: v0.3 provider review-lifecycle duplicate sequence preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の lifecycle sequence duplicate boundary
- Related: [`decisions-v0.3-provider-lifecycle-state-preflight.md`](decisions-v0.3-provider-lifecycle-state-preflight.md)、
  [`decisions-v0.3-review-attestation-sequence-boundary.md`](decisions-v0.3-review-attestation-sequence-boundary.md)

## Context

前段の lifecycle state allowlist は未知 `state` を拒否するが、同じ `review_id` と integer `sequence` を持つ複数 eventの
重複までは検査していなかった。そのため state が許可値で digestも一致していれば、同一 sequence の異なる lifecycle eventが
release identity boundaryへ到達できた。

## Decision

明示 review-lifecycle snapshot の recordが integer `sequence` を持つ場合、`(review_id, sequence)` は snapshot内で一度だけ
現れることを要求する。重複は `duplicate review lifecycle sequence` として fail-closed に拒否する。sequence fieldを持たない既存
provider snapshotは後方互換のため state allowlist・digest検査だけを適用し、sequenceの必須化、rollback ordering、同一 sequenceの
payload比較、append-only reducerはこのsliceへ追加しない。

## Evidence

`test-native-release-identity.py` に、`proposed` と `active` の許可 stateを同じ `review_id` / `sequence: 1` で2件含む fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は安定した duplicate診断で拒否する GREENを確認した。既存の identity・official release・
provider snapshot・stage0 package focused harnessは同じ batchで再実行する。

## Boundary and follow-up

これは duplicate sequence の verified partial sliceである。sequence必須化・rollback/ordering reducer・署名/authentication、live provider
API取得、MCP semantic parity、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有の
Lima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
