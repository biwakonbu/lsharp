# ADR: official multi-target gate の cleanup path traversal 拒否

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05-N9` / `scripts/ci/native-official-release-local.sh`

## Context

official multi-target gate は `SMOKE_ROOT` と hostgen replay lock の一時領域を task-owned
path として扱い、開始時の preflight と終了時の `rm -rf` cleanup を行う。従来の prefix 検査は
`/tmp/lsharp-*` で始まる文字列だけを受け入れていたため、`/tmp/lsharp-owner/../outside`
のような traversal component を含む path が許可され、cleanup 対象が prefix 外へ解決し得た。

## Decision

`native-official-release-local.sh` の cleanup path preflight は、従来の `/tmp/lsharp-*` prefix
制約に加えて、`..` または `.` の path component を含む値を fail-closed で拒否する。
拒否は packaging、provider snapshot、release smoke、Lima VM の起動より前に行い、既存の安全な
task-owned path と lifecycle ownership は変更しない。

## Evidence

- RED: fake two-target official gate に `/tmp/lsharp-owner/../outside` を渡すと、実装前は
  cleanup path traversal を受け入れ、外側 sentinel を削除し得た。
- GREEN: traversal path を `unsafe cleanup path` として拒否し、外側 sentinel を保持する
  回帰契約を `test-native-official-release-snapshots.sh` に追加した。
- `bash scripts/ci/test-native-official-release-snapshots.sh` が pass。
- `bash -n scripts/ci/native-official-release-local.sh scripts/ci/test-native-official-release-snapshots.sh`
  が pass。

## Boundary

これは official gate の task-owned cleanup path に対する lexical traversal boundary である。
provider API/authentication、current-source の Mac/Linux stage0 runtime、packaged artifact、
rollback、Wasm parity は検証しない。`EC-M3-05-N9` は引き続き `[~]` とする。
