# ADR: v0.3 native stage0 release output boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/package-native-stage0-release.sh` の stage0 input/output path boundary
- Related: [`decisions-v0.3-native-stage0-release-provider-snapshot-exclusion.md`](decisions-v0.3-native-stage0-release-provider-snapshot-exclusion.md)、
  [`decisions-v0.3-native-stage0-release-artifact-binding.md`](decisions-v0.3-native-stage0-release-artifact-binding.md)

## Context

stage0 release package builder は、検証済み stage0 directory と archive output directory を別の所有物として
扱う必要がある。しかし output directory が stage0 input directory 自身またはその配下に指定されると、builder が
input package の中へ staging directory を作成してから input 全体を copy する。これにより source package と
generated output が重なり、再帰的な copy や入力汚染を起こし得る。

## Decision

`package-native-stage0-release.sh` は、stage0 package の既存 regular-file/symlink 検査後に、canonical な
stage0 path と output path を比較する。output が stage0 自身またはその配下なら、`mkdir`・staging copy・archive
生成より前に `output directory must be outside native stage0 package` で fail-closed にする。output が stage0 の
外にある通常の relative path は従来どおり受理する。

これは package builder の input/output ownership boundaryに限定し、archive payload、manifest、atomic install、
provider snapshot、identity の各契約は再定義しない。

## Evidence

`test-native-stage0-release-package.sh` に stage0 directory 配下を output に指定する fixtureを追加した。実装前は
builderがnested outputを受理する RED、実装後は安定診断、nested output directory未作成、既存の通常 package成功を
同じ focused harnessで GREEN 確認した。relative output directoryの成功も回帰検証した。

## Boundary and follow-up

これは packaged stage0 の入力/output ownership に関する verified partial sliceである。live provider API/auth取得・
意味検証、current-source Linux runtime、Mac/Linux両 target の packaged provenance/rollback bytes parity は未検証の
ため、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま残す。current HEAD に一致する manifest と
expected replay lock がないため Linux replay、stage regeneration、full build は実行していない。別セッション所有の
Lima/cargo/replay processも変更していない。blockerの再現は次で行う。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
```
