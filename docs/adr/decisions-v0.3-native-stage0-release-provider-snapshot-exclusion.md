# ADR: v0.3 native stage0 release の raw provider snapshot exclusion

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/package-native-stage0-release.sh` の archive staging
- Related: [`decisions-v0.3-provider-snapshot-digest-verification.md`](decisions-v0.3-provider-snapshot-digest-verification.md)、
  [`decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md`](decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md)

## Context

provider trust-store / review-lifecycle snapshot は offline identity verifier の入力であり、公開 stage0 release archiveの
payloadではない。既存の package builderは stage0 directoryを検証した後、その配下を `cp -pR` で archive stagingへ
コピーするため、入力 stage0に canonical raw snapshot filenamesが混入すると、identityの digestだけを残すべき archiveへ
provider bytesも含められた。

## Decision

stage0 release archiveの staging後、root-level の `review-trust-store.snapshot` と `review-lifecycle.snapshot` を明示的に
除外する。既存の `review-evidence-identity.json`、manifest、checksums、stage0 executableは保持し、provider snapshotの
shape、digest、意味検証を追加実装しない。これにより raw provider inputは caller/provider offline boundaryに留まり、公開
archiveには identity projectionだけが残る。

## Evidence

`test-native-stage0-release-package.sh` に private provider snapshot filenamesと固有内容を stage0 inputへ追加した。現行
コードで raw trust-storeが archive listingへ漏れる REDを確認し、staging exclusion後に archiveへ両 snapshotが存在しないこと、
既存 identity付き packageが成功することを GREENで確認した。

## Boundary and follow-up

これは packaged archiveの raw provider material exclusionに限る verified partial sliceであり、live provider API/auth acquisition・
意味検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityを完了した証拠ではない。
M3-04-N1、M3-05-N2、M3-05-N7、M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、
別セッションのLima/cargo/replay processも所有中のため、Linux replay・stage regeneration・full buildは実行しない。再現確認:

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
