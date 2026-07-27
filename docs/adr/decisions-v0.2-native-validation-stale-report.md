# ADR: v0.2 native validation stale report

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-02`、`docs/adr/decisions-v0.2-selfhost-validation-stale-report.md`

## Context

Rust canonical validation と selfhost `App.Cli` / `EmbeddedCli` には、invalidation された review
と、その review が `evaluates` する evidence を stale として伝播し、`unknown=2` で報告する
slice がある。しかし native source-file smoke は stale facts を 0 の fixtureでしか見ておらず、
stale report policy の native observable contract が欠けていた。

## Decision

- `:review`、`:evaluates`、`:invalidates` を含む既存 Rust oracle と同一の stale fixtureを追加する。
- `validate --source <fixture> --format json` は stdout 1行の JSON report、`status: unknown`、
  `trace_gaps: []`、`open_questions: 0`、`independent_reviews: 1`、
  `contradicting_observations: 0`、`stale_reviews: 1`、`stale_evidence: 1`、exit `2` を返す。
- 同じ fixtureの `--format text` は次の固定順を返し、stderr は空にする。

  ```text
  status: unknown
  open-questions: 0
  independent-reviews: 1
  contradicting-observations: 0
  stale-reviews: 1
  stale-evidence: 1
  ```

- unknown/pass/fail、parse/graph/write failure の既存境界と、成功経路の Cargo/Rust/host lsharp
  blocking は維持する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は、
  `VALIDATION_STALE_SOURCE` と stale-report contract が inner runner にないため失敗した。
- GREEN: 同じ focused command は fake Lima/provenance fixtureで
  `Linux native stage0 source-file provenance tests: OK` を返した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh
  scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
  scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash scripts/ci/test-native-selfhost-dev.sh`、`bash scripts/audit_docs.sh` は成功した。
- `git diff --check` は成功した。

## Boundary and follow-up

これは native smoke script の stale report/exit contract を拡張した verified sliceであり、current
source-commit に一致する packaged stage0 の実行、Mac/Linux artifact/runtime、manifest bytes と
standalone Wasm runtime の parity を完了扱いにしない。`TODO.md` の `EC-M2-03` と M3 aggregate は
`[~]` を維持し、別セッションの stage0 replay完了後に同じ stale fixtureの actual executionを
行う。
