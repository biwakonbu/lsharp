# ADR: v0.3 provider review-lifecycle sequence gap preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の同一 review ID に対する native snapshot ordering boundary
- Related: [`decisions-v0.3-provider-lifecycle-sequence-required-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-required-preflight.md)、
  [`decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md`](decisions-v0.3-provider-lifecycle-sequence-rollback-preflight.md)

## Context

native release identity preflight は sequence の欠落・非正数・duplicate・rollbackを検査していたが、
同じ `review_id` の `sequence: 1` の次に `sequence: 3` が現れる gapを受理していた。これは event streamが
途中の lifecycle eventを欠いたまま、release provenanceの意味入力へ到達できる境界だった。

## Decision

同一 `review_id` の sequenced lifecycle snapshotでは、直前 sequenceより大きい値は直前値のちょうど `+1` でなければならない。
`1 → 3` のような gapは `review lifecycle sequence gap` として fail-closedに拒否する。rollback・duplicate・required/positive
の既存診断と、異なる review ID間の sequence orderingはこのsliceで変更しない。

Rust `ReviewLifecycleRegistry` の一般 event reducerは、この元の native-only sliceでは対象外だったため、当時の時点で Rust/native
完全 parityの証拠へ拡大解釈しなかった。後続の parity ADRで同じ gap fixtureを別途検証する。

## Evidence

`test-native-release-identity.py` に `proposed(1) → active(3)` fixtureを追加した。実装前は digest/state一致で exit 0となる RED、
実装後は sequence gap 診断で拒否する GREENを確認した。identity、prepare、official snapshot/replay-lock、release-smoke、stage0
packageの focused harness、syntax、docs auditも再実行した。

## Boundary and follow-up

これは native provider lifecycle snapshotの sequence continuity verified partial sliceである。Rust/native reducer parity、
payload semantics、live provider API/auth取得・意味検証、MCP semantic、current-source Linux runtime、Mac/Linux両 targetの
packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。
current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有のLima/QEMU/replay processもあるため Linux replay・
stage regeneration・full buildは実行しない。blockerの再現 commandは次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
