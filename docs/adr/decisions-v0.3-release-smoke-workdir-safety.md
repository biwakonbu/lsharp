# ADR: v0.3 release smoke cleanup work-directory safety

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh` の cleanup work directory preflight
- Related: [`decisions-v0.3-release-smoke-provider-preflight-order.md`](decisions-v0.3-release-smoke-provider-preflight-order.md)、
  [`decisions-v0.3-native-official-vm-smoke-cleanup.md`](decisions-v0.3-native-official-vm-smoke-cleanup.md)

## Context

release smoke は成功時に `WORK_DIR` を recursive cleanup する。従来は caller が repository root、repository の
shared `target`/`target/ci`、または `/tmp` を指定しても、archive smokeへ進んだ後の cleanup rootとして受け取って
いた。これは task-owned smoke artifact と共有・広域 directoryの所有を分離できず、成功時に広い領域を削除し得る。

## Decision

`release-smoke.sh` は `WORK_DIR` を caller の current directory基準で canonicalizeし、archive/provider workを始める前に
`/`、`/tmp`、`/private/tmp`、repository root、repositoryの `target`、`target/ci` を fail-closed で拒否する。
既存の専用 leaf directory（既定の `target/ci/release-smoke`、または testが作る task-owned temporary leaf）は受理する。
拒否診断は `ERROR: unsafe release smoke work directory` に固定する。

この境界は cleanup ownershipだけを扱い、archive payload、checksum、rollback anchor、provider snapshot、identityの
各契約は変更しない。

## Evidence

`test-release-smoke-provider-snapshots.sh` に repository rootを `WORK_DIR` とし、missing archiveを渡す preflight fixtureを
追加した。実装前は unsafe pathを受理して `archive not found` まで進む RED、実装後は archive accessより前の安定診断を
返す GREENを確認した。既存の valid provider snapshot、rollback payload、provider mismatch、checksum/manifest boundary
fixtureも同じ focused harnessで通過した。

## Boundary and follow-up

これは local release smoke の cleanup path safety に関する verified partial sliceである。live provider API/auth取得・意味
検証、current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parity は未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current HEAD に一致する manifest と expected replay
lock がないため Linux replay、stage regeneration、full build は実行していない。別セッション所有のLima/cargo/replay
processも変更していない。blockerの再現は次で行う。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
