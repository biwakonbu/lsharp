# ADR: packaged release/rollback archive input の regular-file 境界

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh` の release archive / rollback compatibility archive 入力
- Related: `docs/adr/decisions-v0.3-current-source-two-target-runtime.md`

## Context

release-smoke は archive の内容を展開し、rollback compatibility archive は release manifest の
anchor checksum と照合した後に再帰 smoke する。従来は release archive を `-f`、rollback archive
を `-s` で検査していたため、同じ bytes を指す symlink を regular archive input として受け入れ、
path の所有・固定性を確認しないまま展開または hash 対象にできた。

## Decision

release archive と rollback compatibility archive の両入力は、存在する symlink でない regular file
でなければ fail-closed に拒否する。missing input は従来どおり archive not found / required として
拒否し、payload 内の symlink、manifest/anchor、checksum、rollback payload の検証責務は既存契約の
まま変更しない。

## Evidence

- RED: `bash scripts/ci/test-release-smoke-provider-snapshots.sh` は、anchor が期待する basename
  の rollback archive を同じ bytes の symlinkへ置き換える fixtureを受け入れ、`rollback archive
  symlink was accepted` で失敗した。
- GREEN: 同じ fixtureを `regular file without symlink` で拒否し、同 harness が
  `release-smoke provider snapshot tests: OK` となった。
- 検証は offline fixture の release-smoke harness に限定し、共有 Linux VM、stage regeneration、
  full build、外部 provider API/auth は実行していない。

## Boundary and follow-up

これは archive input path の provenance-safe regular-file boundaryだけを閉じる verified partial slice
である。archive bytes の target parity、current-source Linux runtime、Mac/Linux 両 target の packaged
provenance/rollback bytes parity、live provider API/auth acquisition・意味検証は未完了であり、
M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま維持する。current-source manifest/expected
replay lockが現HEADに一致せず、別セッション所有の Lima/QEMU/replayd が稼働中のため Linux replay・
stage regeneration・full buildは実行しない。

blocker の再現・確認 command は次のとおり。

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
ps axww | rg 'limactl|qemu|replayd|cargo|rustc'
```
