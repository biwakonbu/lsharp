# ADR: v0.3 native stage0 release artifact identity binding

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `package-native-stage0-release.sh` の packaged stage0 identity preflight
- Related: [`decisions-v0.3-release-identity-gate.md`](decisions-v0.3-release-identity-gate.md)、
  [`decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md`](decisions-v0.3-native-stage0-release-embedded-identity-provider-preflight.md)

## Context

stage0 release package は provider snapshot digest、source commit、identity schema を検証していたが、
`artifact_digest` を package 内の実 compiler bytesへ突き合わせていなかった。そのため、provider inputと
source commitが正しくても、別 artifactを表す identityを stage0 archiveへ格納できた。

## Decision

`package-native-stage0-release.sh` は stage0 manifestの `compiler` relative pathを解決し、既存の
`verify-native-release-identity.py` へ `--artifact` として渡す。identityの artifact digestが packaged
compiler bytesと一致しない場合は archive作成前に fail-closed とする。provider API/auth取得や署名・lifecycleの
意味検証はこの境界へ追加せず、既存の明示 snapshot/verifier boundaryへ委譲する。

## Evidence

`test-native-stage0-release-package.sh` の fixture identityを実 compiler digestへ束縛し、別 digestを与えた
REDで archive生成を確認した後、同じ harnessで mismatchの拒否、archive未生成、正しい identityの package成功を
GREENで確認した。既存 stage0 package payload、provider snapshot mismatch、embedded identity preflightも同じ
focused harnessで維持した。

## Boundary and follow-up

これは packaged stage0 artifact provenanceの partial sliceである。current-source Linux runtime、provider
API/auth取得・意味検証、Mac/Linux両 targetの packaged bytes parity、rollback parityは未検証のため、
M3-05-N2 / M3-05-N7 / M3-05-N9 と M3-04-N1 は `[~]` のまま残す。current-source manifest/expected replay lockが
揃わない状態と別セッション所有のLinux replayは変更しない。再現 command は
`current_head="$(git rev-parse --verify HEAD)"; find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'` と
`find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)` である。
