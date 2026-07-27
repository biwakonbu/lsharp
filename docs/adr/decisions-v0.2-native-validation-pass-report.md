# ADR: v0.2 native validation complete-graph pass report

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M3-01`、`EC-M3-02`、`docs/adr/decisions-v0.2-selfhost-validation-pass-status.md`

## Context

native source-file smoke は canonical fixture の `unknown`、duplicate-node、manifest write failure
を検査していたが、complete graph が `pass=0` として report/exit を返すことを要求していなかった。
Rust-host actual Wasm の Cli/EmbeddedCli pass evidenceだけでは、native stage0 の status projection
drift を検出できない。

## Decision

- independent review を含む complete graph fixtureを native source-file smoke に追加する。
- `validate --source <fixture> --format json` は `status: pass`、全 metrics zero except
  `independent_reviews: 1`、exit `0` を返すことを要求する。
- 同じ fixtureの `--format text` は次の固定順を返す。

  ```text
  status: pass
  open-questions: 0
  independent-reviews: 1
  contradicting-observations: 0
  stale-reviews: 0
  stale-evidence: 0
  ```

- unknown、fail、write-failure の既存境界と、成功経路の Cargo/Rust/host lsharp blocking は維持する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は、complete
  graph fixture/contract がなく失敗した。
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
`[~]` を維持し、別セッションの stage0 replay完了後に同じ fixtureの actual executionを行う。
