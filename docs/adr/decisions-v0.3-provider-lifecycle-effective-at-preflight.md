# ADR: v0.3 provider review-lifecycle effective_at preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の lifecycle timestamp input boundary
- Related: [`decisions-v0.3-review-lifecycle-effective-timestamp.md`](decisions-v0.3-review-lifecycle-effective-timestamp.md)、
  [`decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md)

## Context

native release identity preflight は review-lifecycle の state、duplicate sequence、sequence rollback を検査していたが、
`effective_at` が存在する recordの不正な日付を受理していた。digestが一致していても、Rust lifecycle boundaryと共有すべき
strict UTC/calendar semanticsをnative provider inputへ伝播できていなかった。

## Decision

review-lifecycle recordに `effective_at` fieldが存在する場合、既存の `is_valid_utc_timestamp` を使い、
`YYYY-MM-DDTHH:MM:SSZ`、実在する Gregorian 日付、秒 `00..59` を満たさない値を
`review lifecycle effective_at must be a strict UTC timestamp` として fail-closed に拒否する。
既存の sequenceなし snapshotとの互換性を保つため、field自体の必須化はこのsliceに追加しない。

## Evidence

`test-native-release-identity.py` に `2024-02-30T00:00:00Z` を含む lifecycle fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は strict UTC timestamp診断で拒否する GREENを確認した。

## Boundary and follow-up

これは native provider inputの `effective_at` shape/calendar preflightに限る verified partial sliceである。
sequence reducer、state transition、effective-time ordering、署名/authentication、live provider API取得、MCP semantic parity、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
