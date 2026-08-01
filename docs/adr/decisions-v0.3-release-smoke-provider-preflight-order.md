# ADR: v0.3 release smoke の provider preflight order

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh` の review provider snapshot preflight
- Related: [`decisions-v0.3-provider-snapshot-digest-verification.md`](decisions-v0.3-provider-snapshot-digest-verification.md)、
  [`decisions-v0.3-native-stage0-release-smoke-snapshot-wiring.md`](decisions-v0.3-native-stage0-release-smoke-snapshot-wiring.md)

## Context

release smoke は `RELEASE_REVIEW_TRUST_STORE` と `RELEASE_REVIEW_LIFECYCLE` を native archive の identity verifierへ渡す。
従来は provider snapshot の all-or-none preflightが native-only archiveの存在確認と展開の後にあり、欠落した片方の入力でも
archive boundaryへ先に到達し、provider入力ではなく archive error が最初の診断になっていた。

## Decision

release smoke の provider snapshot preflightを archive pathの存在確認より前に実行する。不完全または空の provider inputは
all-or-none / non-empty の既存診断で fail-closed とし、archive lookup、展開、release smoke work directory作成、binary/runtimeへ
進ませない。valid provider snapshotsの verifier forwardingと rollback smoke時の provider env clearは変更しない。

## Evidence

`test-release-smoke-provider-snapshots.sh` に、欠落 archiveと trust-storeのみを指定する fixtureを追加した。現行コードで
`archive not found` が provider診断より先に返る REDを確認し、preflight移動後に all-or-none診断、work directory未作成を
GREENで確認した。既存の valid snapshot、tampered digest、rollback manifest/anchor/checksum focused casesも同じ harnessで
通過した。

## Boundary and follow-up

これは provider input failure-order の verified partial sliceであり、live provider API/auth acquisition・semantic verification、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityを完了した証拠ではない。M3-04-N1、
M3-05-N2、M3-05-N7、M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別
セッションのLima/cargo/replay processも所有中のため、Linux replay・stage regeneration・full buildは実行しない。再現確認:

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
