# ADR: v0.3 native stage0 embedded identity conflict boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `package-native-stage0-release.sh` の embedded/explicit identity binding
- Related: [`decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md`](decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md)、
  [`decisions-v0.3-native-stage0-release-artifact-binding.md`](decisions-v0.3-native-stage0-release-artifact-binding.md)

## Context

stage0 directoryに `review-evidence-identity.json` がある場合、release package callerが別の明示 identityを
渡しても、従来は明示値で embedded payloadを上書きして archiveを作成できた。source commit、artifact digest、
provider snapshotが個別に正しくても、stage0入力が表す provenanceとpackaged provenanceが分岐する。

## Decision

明示 identityを verifierで canonical projectionした後、embedded identityが存在する場合はJSON objectとして
比較する。一致しない場合は `embedded review evidence identity conflicts with explicit input` として
archive作成前に fail-closed とする。一致時は従来どおり canonical projectionを package manifest/fileへ保持する。
provider API/auth取得や署名・lifecycle意味検証は追加せず、既存の明示 snapshot/verifier boundaryへ委譲する。

## Evidence

`test-native-stage0-release-package.sh` へ、compiler artifact digestとprovider snapshotが正しいまま subject
digestだけ異なる embedded/explicit identity fixtureを追加した。現行コードで archiveが生成される REDを確認後、
一致ケースの package成功、不一致時の安定診断、archive未生成を同じ focused harnessでGREEN確認した。

## Boundary and follow-up

これは packaged stage0 provenanceの conflict boundaryに限る verified partial sliceである。provider API/auth取得・
意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged bytes parity、rollback parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9は `[~]` のまま残す。current-source manifest/expected replay lockが
現HEADに一致しないためLinux replayは未実行で、別セッション所有のLima/cargo/replayは変更しない。
