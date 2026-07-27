# ADR: v0.2 native validation contradiction fail report

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-01`、`EC-M3-02`、`docs/adr/decisions-v0.2-selfhost-validation-pass-status.md`

## Context

native source-file smoke は unknown、parse/graph error、manifest write failure、complete graph の
pass を検査していたが、検証結果が矛盾した場合に report を保持した `fail=1` 境界を要求して
いなかった。Rust-host actual Wasm の `EmbeddedCli` には contradiction fixture の evidence が
あるため、native source-file lane でも判定 failure と入力/出力 failure を混同しない契約が必要
だった。

## Decision

- `:outcome "contradicted"` と `:contradicts` edge を含む独立 review fixtureを追加する。
- `validate --source <fixture> --format json` は stdout 1行の JSON report、`status: fail`、
  `trace_gaps: []`、`open_questions: 0`、`independent_reviews: 1`、
  `contradicting_observations: 1`、stale facts `0`、exit `1` を返す。
- 同じ fixtureの `--format text` は次の固定順を返し、stderr は空にする。

  ```text
  status: fail
  open-questions: 0
  independent-reviews: 1
  contradicting-observations: 1
  stale-reviews: 0
  stale-evidence: 0
  ```

- parse/graph/write failure の report なし境界、unknown/pass の既存 status、成功経路の
  Cargo/Rust/host lsharp blocking は維持する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は、
  `VALIDATION_FAIL_SOURCE` と fail-report contract が inner runner にないため失敗した。
- GREEN: 同じ focused command は fake Lima/provenance fixture で
  `Linux native stage0 source-file provenance tests: OK` を返した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh
  scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
  scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`、
  `bash scripts/ci/test-native-selfhost-dev.sh`、`bash scripts/audit_docs.sh` は成功した。
- `git diff --check` は成功した。

## Boundary and follow-up

これは native smoke script の report/exit contract を拡張した verified sliceであり、current
source-commit に一致する packaged stage0 の実行、Mac/Linux artifact/runtime、manifest bytes と
standalone Wasm runtime の parity を完了扱いにしない。`TODO.md` の `EC-M2-03` と M3 aggregate は
`[~]` を維持し、別セッションの stage0 replay完了後に同じ contradiction fixtureの actual
executionを行う。
