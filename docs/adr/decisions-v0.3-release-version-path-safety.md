# ADR: v0.3 release version path safety

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/release.sh` の native release version preflight
- Related: [`decisions-v0.3-fetch-stage0-install-directory-safety.md`](decisions-v0.3-fetch-stage0-install-directory-safety.md)、
  [`decisions-v0.3-native-stage0-release-output-boundary.md`](decisions-v0.3-native-stage0-release-output-boundary.md)

## Context

`release.sh` は `VERSION` を archive directory、archive filename、manifest metadataへ展開する。従来は targetだけを
検証してから `DIST_DIR/${ARCHIVE_NAME}` を作成していたため、`VERSION` に `/` を含む値を渡すと、versionを単一の
release scalarとして扱わず、予期しない下位 directoryを作成してから別の入力エラーへ進んでいた。

## Decision

`release.sh` は target validation直後、release output directory作成より前に `VERSION` を ASCII letters、digits、
dot、underscore、hyphenだけへ限定する。拒否診断は `ERROR: version must contain only` に固定する。

この境界は release version namespaceだけを扱い、archive entry、manifest schema、checksum、rollback anchor、provider
identity、atomic install、stage0 package input/output の各契約を変更しない。

## Evidence

`test-native-release-identity.py` に `VERSION=v1/unsafe` の fixtureを追加した。実装前は version validationを通過して
`NATIVE_ONLY_PROGRAM` エラーまで進み、temporary `DIST_DIR` を作成する REDだった。実装後は output directory作成前に
安定診断で拒否し、temporary distが存在しない GREENを確認した。既存の native release identity、provider digest、
package/archive harnessは同じ focused batchで再確認する。

## Boundary and follow-up

これは release version namespace の verified partial sliceである。live provider API/auth acquisition・意味検証、
current-source Linux runtime、Mac/Linux両 targetの packaged provenance/rollback bytes parityは未検証のため、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current HEAD に一致する manifest と expected replay
lockがなく、別セッション所有のLima/cargo/replay processもあるため Linux replay、stage regeneration、full buildは
実行しない。blockerの再現は次で行う。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
