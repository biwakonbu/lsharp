# ADR: embedded native stage0 identity の provider snapshot preflight

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `EC-M3-05-N2` / `EC-M3-05-N7` / `scripts/ci/package-native-stage0-release.sh`
- Related: [`decisions-v0.3-native-official-provider-identity-schema-preflight.md`](decisions-v0.3-native-official-provider-identity-schema-preflight.md)

## Context

release package builder は stage0 directory 内の `review-evidence-identity.json` を manifestへ
取り込める一方、provider snapshotを明示しない direct invocationでも archiveを作れる余地があった。
これは identityを持つ packaged inputをprovider provenanceなしで配布する境界を曖昧にする。

## Decision

stage0 directoryに identityが埋め込まれている場合、release package builderは明示的な identity入力と
trust-store / review-lifecycle snapshotの両方が揃うまで packagingを開始しない。揃った場合の schema、
source commit、digest、timestamp検証は既存 `verify-native-release-identity.py`へ委譲し、identityの
schemaやprovider意味論を再実装しない。snapshot未指定でidentityを持たない旧 archiveは既存の互換境界として残す。

## Evidence

- RED: embedded identityを持つ stage0をprovider snapshotなしで渡すと、実装前は release archiveを生成した。
- GREEN: 同じ fixtureが `embedded review evidence identity requires explicit provider snapshots` で
  packaging前に停止し、archiveを残さない。
- `bash scripts/ci/test-native-stage0-release-package.sh`、official snapshot/replay-lock/provider-snapshot
  focused harness、shell syntax、diff check、docs auditを通過。

## Boundary

これは packaged identity inputの offline preflightに限る verified partial sliceである。live provider/auth
取得・署名意味検証、current-source Linux runtime、両 targetの packaged bytes parity、rollback/Wasm parityは
未検証であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9は`[~]`のまま維持する。
