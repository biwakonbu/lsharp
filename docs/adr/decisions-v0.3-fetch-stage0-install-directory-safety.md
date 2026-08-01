# ADR: v0.3 fetch-stage0 install directory safety

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/fetch-stage0.sh` の stage0 install destination preflight
- Related: [`decisions-v0.3-release-smoke-workdir-safety.md`](decisions-v0.3-release-smoke-workdir-safety.md)、
  [`decisions-v0.3-native-stage0-release-output-boundary.md`](decisions-v0.3-native-stage0-release-output-boundary.md)

## Context

`fetch-stage0.sh` は検証済み package を `STAGE0_DIR` へ atomic に置き換える。従来の install 関数が拒否するのは
`/` と文字列としての `.` だけだったため、repository root、repository の共有 `target` / `target/ci`、または
system temporary root を destination に指定すると、checksum/package 検証のための network/curl と temporary
workspace の作成を先に許していた。既存の package validation と atomic restore の責務だけでは、install destination
の所有範囲を fail-closed にできない。

## Decision

`fetch-stage0.sh` は `STAGE0_DIR` を作成せずに canonicalize し、release URL validation、temporary workspace、curl
より前に `/`、`/tmp`、`/private/tmp`、repository root、repository の `target`、`target/ci` を拒否する。
未作成の親を含む path を解決し、既存の leaf stage0 directory と symlink leaf の install/atomic rollback semantics
は変更しない。拒否診断は `ERROR: unsafe stage0 install directory` に固定する。

この境界は fetch の install destination ownershipだけを扱い、release smoke cleanup、package input/output ownership、
archive payload、checksum、provider snapshot、identity、atomic install/restore の各契約を再実装しない。

## Evidence

`test-fetch-stage0-provider-url.sh` に protected destination fixtureを追加した。実装前は repository root を指定しても
curl が呼ばれ、checksum file failureへ進む REDだった。実装後は repository root、`target`、`target/ci`、`/tmp`、
`/private/tmp` の全 fixtureが stable diagnosticを返し、curl未実行と repository root未変更を確認する GREENになった。
既存の provider URL、archive provenance、atomic install harnessは同じ focused batchで再確認する。

## Boundary and follow-up

これは fetch install destination safetyの verified partial sliceである。live provider API/auth acquisition・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current HEAD に一致する manifest と expected replay
lockがなく、別セッション所有のLima/cargo/replay processもあるため Linux replay、stage regeneration、full buildは
実行しない。blockerの再現は次で行う。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
