# ADR: provider lifecycle future `effective_at` freshness boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の明示 provider lifecycle snapshot
- Related: [`decisions-v0.3-provider-lifecycle-effective-at-ordering-parity.md`](decisions-v0.3-provider-lifecycle-effective-at-ordering-parity.md)

## Context

provider snapshot の raw bytes と digest が一致していても、lifecycle event の
`effective_at` が release identity の caller-provided `now` より未来なら、その snapshot は
現在時点の authorization/freshness evidence として扱えない。既存 preflight は timestamp の
形式、同一 review 内の sequence 順、および snapshot digest を検査していたが、future event を
受理していた。

## Decision

`effective_at` が存在する lifecycle recordは strict UTC timestampとして検証した後、同じ identityの
canonical `now` と比較する。`effective_at > now` の recordは
`review lifecycle effective_at is after identity now` で fail-closed に拒否する。
`effective_at` の省略、同一 review内の ordering、provider snapshot digest、attestationの
`issued_at`/`expires_at`、live provider取得/authenticationはこのsliceで変更しない。

## Evidence

- RED: `test-native-release-identity.py NativeReleaseIdentityTest.test_rejects_review_lifecycle_effective_at_after_identity_now` は、
  `effective_at=2026-08-16T00:00:00Z` と identity `now=2026-08-15T00:00:00Z` を実装前に exit `0` として受理した。
- GREEN: 同じ fixtureを `review lifecycle effective_at is after identity now` で拒否する focused testを通過した。
- このsliceは offline native release identity boundaryであり、provider API/auth取得や署名の
  暗号学的検証を完了扱いにしない。

## Boundary and follow-up

これは provider lifecycle snapshot の caller-clock freshness preflightに限る verified partial sliceである。
live provider API/auth acquisition・意味検証、完全な transition matrix/reducer、current-source Linux runtime、
Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 /
M3-05-N7 / M3-05-N9 は `[~]` のまま維持する。current-source manifest/expected replay lockが現HEADに一致せず、
別セッション所有のLima/QEMU/replaydも稼働中のため Linux replay・stage regeneration・full buildは実行しない。
blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
