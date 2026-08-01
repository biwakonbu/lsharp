# ADR: packaged rollback executable version の manifest parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/release-smoke.sh` の native-only rollback compatibility smoke
- Related: [`decisions-v0.3-packaged-archive-input-regular-file.md`](decisions-v0.3-packaged-archive-input-regular-file.md)

## Context

native-only release smoke は release manifest の version と packaged App.Cli の `--version` output を
照合するが、rollback compatibility archive の再帰 smoke では `EXPECTED_ROLLBACK_VERSION` を manifest
preflight にだけ渡し、`VERSION` を設定していなかった。そのため rollback manifest、anchor checksum、
payload checksum が全て正しくても、rollback executable が別 version を報告する package を受け入れ得た。

## Decision

rollback archive を再帰 smoke するときは、検証済み rollback manifest の `version` を nested
`release-smoke.sh` の `VERSION` として渡す。これにより rollback executable の `--version` output が
`lsharp <manifest version without v>` と一致しなければ fail-closed になる。manifest/anchor/checksum/
payload、archive input regular-file、release archive version namespace の既存責務は変更しない。

## Evidence

- RED: `bash scripts/ci/test-release-smoke-provider-snapshots.sh` の rollback fixtureで `lsharp --version`
  だけを `9.9.9` に変更し、manifest version、anchor SHA、checksums を再生成しても、実装前は
  `rollback executable version mismatch was accepted` となった。
- GREEN: 同じ fixtureを nested smoke の `packaged CLI version mismatch` で拒否し、harness が
  `release-smoke provider snapshot tests: OK` となった。
- これは同一 offline release/rollback fixtureによる differential boundary であり、外部 provider API/auth、
  Linux VM、stage regeneration、full build は実行していない。

## Boundary and follow-up

これは rollback executable version output と rollback manifest version の parityだけを閉じる verified
partial sliceである。current-source Linux runtime、Mac/Linux 両 target の packaged/rollback bytes parity、
live provider API/auth acquisition・意味検証は未完了であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は
`[~]` のまま維持する。current-source manifest/expected replay lockが現HEADに一致せず、別セッション所有の
Lima/QEMU/replayd が稼働中のため Linux replay・stage regeneration・full buildは実行しない。

再現・所有状態確認 command:

```bash
current_head="$(git rev-parse --verify HEAD)"
find /tmp /Users/biwakonbu/github/tmp -maxdepth 5 -type f -name manifest.json -path '*lsharp*'
find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 \( -name 'lsharp-native-linux-x86-hostgen-vm-*' -o -name '*.lock' \)
ps axww | rg 'limactl|qemu|replayd|cargo|rustc'
```
