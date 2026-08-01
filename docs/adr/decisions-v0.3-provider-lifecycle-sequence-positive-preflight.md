# ADR: v0.3 provider review-lifecycle sequence positive preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の sequenced lifecycle record boundary
- Related: [`decisions-v0.3-review-attestation-sequence-boundary.md`](decisions-v0.3-review-attestation-sequence-boundary.md)、
  [`decisions-v0.3-provider-lifecycle-proposed-state-self-transition-preflight.md`](decisions-v0.3-provider-lifecycle-proposed-state-self-transition-preflight.md)

## Context

Rust canonical lifecycle は `sequence >= 1` を要求しているが、native release identity preflight は
`sequence: 0` を含む lifecycle recordを、stateとdigestが一致していれば受理していた。これにより、
native provider inputだけが schema/modelの lower-bound と異なる値を通過できた。

## Decision

`sequence` fieldが存在する review lifecycle recordは、booleanではない正の整数でなければならない。
`0`、負数、boolean、その他の型は `review lifecycle sequence must be a positive integer` として
fail-closedに拒否する。sequence fieldを持たない既存の非sequenced input、state transition、duplicate/rollback
の契約はこのsliceでは変更しない。

## Evidence

`test-native-release-identity.py` に `sequence: 0` の初期 `proposed` record fixtureを追加した。
実装前は digest一致で exit 0となる RED、実装後は positive-integer 診断で拒否する GREENを確認した。

## Boundary and follow-up

これは native provider inputの sequence lower-bound verified partial sliceである。sequence field必須化、
完全な transition matrix、payload reducer、effective-time ordering、署名/authentication、live provider API取得、
MCP semantic parity、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは
未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致せず、別セッション所有のLima/cargo/replay processもあるため Linux replay・stage regeneration・full buildは実行しない。
blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
