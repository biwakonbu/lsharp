# ADR: v0.2 native validation manifest write failure

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-02`、`EC-M3-05`、`docs/adr/decisions-ec-m3-embedded-manifest-emission.md`

## Context

Rust-host actual Wasm の `EmbeddedCli` では、`--emit-manifest` の親ディレクトリが存在しない
場合に manifest write failure を fail-closed で返す契約がある。一方、native source-file smoke
は canonical manifest bytes と duplicate-node rejection だけを検査し、同じ filesystem failure
で report や artifact を誤って残さないことを要求していなかった。

## Decision

- EC-M3-01 の canonical source fixture を、存在しない親ディレクトリへの
  `validate --source ... --format json --emit-manifest` に再利用する。
- manifest write が失敗した場合は、安定診断 `source validation manifest write failed`、exit `1`、
  空の stdout、manifest artifact なしを要求する。
- duplicate-node の source validation error と成功時の JSON/text report は既存の境界として維持する。
- この smoke script の契約は atomic/durable replacement、source provenance、実 stage0 artifact/runtime
  の完了証拠とは分離する。

## Evidence

- RED: write-failure 要求を追加した `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  は、inner smoke script に契約がなく失敗した。
- GREEN: 同じ focused command は fake Lima/provenance fixture で
  `Linux native stage0 source-file provenance tests: OK` を返した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh
  scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
  scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は成功した。
- `bash scripts/ci/test-native-selfhost-dev.sh` と `bash scripts/audit_docs.sh` は成功し、
  `git diff --check` も成功した。

## Boundary and follow-up

これは native source-file smoke の要求契約を拡張した verified sliceであり、current source-commit に
一致する stage0 artifact での実行、Mac Apple Silicon / Linux x86_64 の runtime、atomic/durable
filesystem、provenance mismatch、rollback は未検証である。`TODO.md` の `EC-M2-03` と M3 aggregate は
`[~]` のまま維持し、次は実 stage0 で同じ write failure と成功 report/manifest bytesを実行する。
