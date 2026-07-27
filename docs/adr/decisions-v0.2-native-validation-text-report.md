# ADR: v0.2 native validation text report contract

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`, `scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh`
- Related: `EC-M3-02`、`EC-M3-04`、`docs/development/planning/v0.2-milestone-03.md`

## Context

Rust-host actual Wasm では `validate --source --format text` の status、件数、行順、exit code を
確認済みだが、native stage0 source-file smoke は canonical fixture を JSON mode だけで検査していた。
このままでは、native runner が text mode を未接続のままでも smoke contract を通過できる。

## Decision

- EC-M3-01 の canonical source fixture を native source-file smoke の text modeにも再利用する。
- `validate --source "$VALIDATION_SOURCE" --format text` は `unknown` report と exit `2` を返し、次の
  deterministic line orderを維持する。

  ```text
  status: unknown
  open-questions: 1
  independent-reviews: 0
  contradicting-observations: 0
  stale-reviews: 0
  stale-evidence: 0
  ```

- Linux wrapper の contract test は inner source-file smoke にこの boundary が存在することを確認する。
- JSON report、manifest bytes、duplicate-node の fail-closed boundary は既存のまま維持する。
- この slice は smoke script の要求契約であり、実 stage0 artifact の実行成功や Mac/Linux runtime parityを
  完了扱いにしない。

## Evidence

- RED: text boundary の要求を追加した `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  は、既存 script に該当する command がなく失敗した。
- GREEN: 同じ focused command は fake Lima/provenance fixture で `Linux native stage0 source-file
  provenance tests: OK` を返した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh
  scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh
  scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は成功した。
- `bash scripts/ci/test-native-selfhost-dev.sh` は `native selfhost dev runner tests: OK` を返した。
- `git diff --check` は成功した。

## Boundary and follow-up

current checkout で source-commit に一致する packaged native stage0 artifactを取得していないため、
Mac Apple Silicon / Linux x86_64 の実 runner、Wasm artifact/runtime、fallback negative gate は未検証である。
`TODO.md` の `EC-M2-03`、M3 の native stage0 parity、two-target matrix は `[~]` のまま維持し、次は
同じ fixture の実 stage0 executionで text/JSON report、exit、manifest bytesを比較する。
