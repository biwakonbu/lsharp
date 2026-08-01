# ADR: v0.3 native official release gate の replay lock precedence

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `native-official-release-local.sh` の Linux hostgen replay lock preflight
- Related: [`decisions-v0.3-native-official-provider-identity-preflight.md`](decisions-v0.3-native-official-provider-identity-preflight.md)、
  [`decisions-v0.3-native-official-vm-lifecycle-cleanup.md`](decisions-v0.3-native-official-vm-lifecycle-cleanup.md)

## Context

公式 release gate は Mac/Linux の package、provider snapshot、source smoke、rollback smokeを一つの入口から
実行する。別セッションが Linux hostgen replayを所有している間は、その成果物・VM・lockを gateが共有してはならない。
従来は replay lockの検査が provider snapshot / identity preflightの後にあり、provider入力が不完全な場合に、保持中の
replay lockより先に別の入力エラーを返していた。これは処理を開始しなくても、現在の所有境界を最初の診断として観測できない。

## Decision

`HOSTGEN_REPLAY_LOCK_DIR` の安全性と live ownerを、provider snapshot、identity、source smoke evidenceの検証より先に
preflightする。保持中または owner不明の lockは既存の exit `90` と owner/artifact/VM path 診断で fail-closed とし、
`DIST_DIR` を作成せず、後続の provider/package/fetch/smoke/Lima 境界へ到達させない。lockが無い場合の既存処理順序と
stale lockを自動削除しない方針は変更しない。

## Evidence

`test-native-official-release-replay-lock.sh` に、live lockと同時に provider snapshotを指定するが identityを欠く fixtureを
追加した。現行コードでは provider identity診断が先に返る REDを確認し、preflightを入力検証より前へ移した後、lock ownerを
含む exit `90` の診断、release output未生成を GREEN で確認した。

## Boundary and follow-up

これは operator ownership / failure-order の verified partial sliceであり、provider API/auth取得・意味検証、current-source
Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityを完了した証拠ではない。M3-04-N1、M3-05-N2、
M3-05-N7、M3-05-N9 は `[~]` のまま残す。current-source manifest/expected replay lockが現HEADに一致せず、別セッションの
Lima/cargo/replay processも所有中のため、Linux replay・stage regeneration・full buildは実行しない。再現確認:

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
