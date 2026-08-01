# ADR: v0.3 provider review-lifecycle sequence required preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の明示 lifecycle snapshot record boundary
- Related: [`decisions-v0.3-provider-lifecycle-sequence-positive-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-positive-preflight.md)、
  [`decisions-v0.3-review-attestation-sequence-boundary.md`](decisions-v0.3-review-attestation-sequence-boundary.md)

## Context

native release identity preflight は、`sequence` が存在する場合の lower boundを検査していたが、
`review_id` と allowlisted `state` だけを持つ lifecycle recordを受理していた。Rust canonical lifecycle
eventとreview provenance schemaが sequenceを識別子として要求するため、sequence欠落を未検証のまま通すと、
同じ provider lifecycle inputが native boundaryだけで異なる意味になる。

## Decision

明示 `--review-lifecycle` snapshotの各 JSON object recordは `sequence` fieldを必須とする。欠落した recordは
`review lifecycle sequence is required` として fail-closedに拒否する。sequenceの正数・型検査、既存の state allowlist、
initial/terminal/self-transition、duplicate/rollback/effective_at 契約は維持し、このsliceでは変更しない。

関連する offline release harness の lifecycle fixtureにも `sequence: 1` を明示し、同じ verifier boundaryを通す。

## Evidence

`test-native-release-identity.py` に sequenceなしの初期 `proposed` record fixtureを追加した。実装前は digest一致で
exit 0となる RED、実装後は required-field 診断で拒否する GREENを確認した。identity preparation、official snapshot、
replay-lock、release-smoke、stage0 package の fixtureも sequence付きへ更新し、focused gateを再実行した。

## Boundary and follow-up

これは native provider lifecycle snapshotの sequence presence verified partial sliceである。provider API/auth取得・意味検証、
完全な transition matrix/reducer、MCP semantic parity、current-source Linux runtime、Mac/Linux両 targetの packaged
provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replay processもあるため
Linux replay・stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
