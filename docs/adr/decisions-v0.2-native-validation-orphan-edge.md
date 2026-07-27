# ADR: v0.2 native validation orphan edge rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-01`、`EC-M3-02`、`docs/adr/decisions-v0.2-native-validation-fail-report.md`

## Context

Source graph の typed edge が未登録 node を参照すると、reportを生成して status を返す前に
入力を拒否しなければならない。Rust source adapter は `MissingNodeReference` としてこの境界を
持ち、selfhost `App.Cli` は stable source graph error code `5` を stderr に出す。一方 native
source-file smoke は duplicate node だけを検査しており、orphan endpoint の no-report/no-manifest
境界を固定していなかった。

## Decision

- `:motivates "intent:checkout/missing" "claim:checkout/rejects"` の orphan fixtureを追加する。
- `validate --source <fixture> --format json --emit-manifest <path>` は exit `1`、stderr の
  `source validation error:5`、空 stdout、manifest fileなしを返す。
- reportを保持する contradiction/stale/unknown と、入力を拒否する duplicate/write/orphan の
  境界を分離して維持する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は、
  `VALIDATION_ORPHAN_SOURCE` と orphan error contract が inner runner にないため失敗した。
- GREEN: 同じ focused command は fake Lima/provenance fixtureで
  `Linux native stage0 source-file provenance tests: OK` を返した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh
  scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
  scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash scripts/ci/test-native-selfhost-dev.sh`、`bash scripts/audit_docs.sh` は成功した。
- `git diff --check` は成功した。

## Boundary and follow-up

これは native source-file smoke の missing-node error contract を拡張した verified sliceであり、
current source-commit に一致する packaged stage0 の実行、Mac/Linux artifact/runtime、manifest
bytes と standalone Wasm runtime の parity を完了扱いにしない。`TODO.md` の `EC-M2-03` と M3
aggregate は `[~]` を維持し、同じ orphan fixtureの actual stage0 execution を replay完了後に
行う。
