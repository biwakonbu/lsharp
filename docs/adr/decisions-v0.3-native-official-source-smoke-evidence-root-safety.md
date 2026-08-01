# ADR: v0.3 native official source-smoke evidence root safety

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/native-official-release-local.sh` の source-smoke evidence output root preflight
- Related: [`decisions-v0.3-release-smoke-workdir-safety.md`](decisions-v0.3-release-smoke-workdir-safety.md)、
  [`decisions-v0.3-fetch-stage0-install-directory-safety.md`](decisions-v0.3-fetch-stage0-install-directory-safety.md)

## Context

official release gate は `NATIVE_OFFICIAL_SOURCE_SMOKE_EVIDENCE_ROOT` 配下へ Mac/Linux source-smoke evidenceを保存する。
従来は absolute、non-root、non-symlink、release smoke root外であることだけを確認していたため、repository rootや
shared `target` / `ci-artifacts` / `dist` / `stage0` を指定しても gateが受理し、後段の target evidence writerが共有
checkoutへ書き込める境界だった。

## Decision

source-smoke evidence rootは raw absolute/non-symlink policyを維持したまま canonicalizeし、package/output/runtime work
より前に `/`、`/tmp`、`/private/tmp`、repository root、repositoryの `target`、`target/ci`、`ci-artifacts`、`dist`、
`stage0`、および cleaned release smoke root配下を拒否する。task-owned non-existing leafは受理する。

この境界は source-smoke evidence output namespaceだけを扱い、package input/output、cleanup、provider snapshot、archive、
manifest、identity、runtime semantics、atomic installの契約を変更しない。

## Evidence

`test-native-official-release-snapshots.sh` に fake official gateで repository rootを evidence rootに指定する fixtureを追加した。
実装前は evidence preflightを通過して missing artifact errorへ進み、DIST_DIRを作成する REDだった。実装後は安定した
`source smoke evidence root is a protected shared path` 診断を返し、DIST_DIRを作成しない GREENを確認した。既存の
Mac/Linux source-smoke evidence propagation、provider snapshot propagation、Lima lifecycle failure casesも同じ harnessで
通過した。

## Boundary and follow-up

これは source-smoke evidence output ownership の verified partial sliceである。live provider API/auth acquisition・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、M3-04-N1 /
M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current HEAD に一致する manifest と expected replay lockがなく、
別セッション所有のLima/cargo/replay processもあるため Linux replay、stage regeneration、full buildは実行しない。blockerの
再現は次で行う。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
