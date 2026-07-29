# ADR: v0.2 native validation の failed independent review gate

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: EC-M2-03, `docs/adr/decisions-v0.2-validation-independent-review-outcome.md`

## Context

Rust canonical validation は `outcome=pass` かつ `independence=independent-review` の evidence だけを
独立 review gate の成立として数える。native source-file smoke には pass、contradiction、stale の
fixture はあったが、failed independent review が complete graph を満たしても gate を成立させない
契約を独立した fixture として固定していなかった。

## Decision

complete graph に `method=review`、`outcome=fail`、`independence=independent-review` の evidence を
持つ source fixture を追加する。native smoke は次を必須とする。

- JSON/text の report status は `unknown`
- report の exit code は `2`
- `independent_reviews` は `0`
- stderr は空
- trace gap、open question、contradiction、stale の件数はすべて `0`

report を保持する判定 failure と parse/graph/write の diagnostic-only failure は引き続き分離する。

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` は、failed-review
  fixture の required marker がないため失敗した。
- GREEN: 同じ static/provenance harness は fixture と JSON/text assertions の追加後に通過した。
- `bash -n scripts/ci/native-selfhost-dev-source-file-smoke.sh scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh`
  と `git diff --check` を通過した。

この evidence は fake Lima/provenance harness の contract 検証であり、current source-commit に一致する
実 stage0 artifact/runtime、selfhost/native/MCP parity、または両 supported target の実行証跡ではない。

## Boundary / follow-up

EC-M2-03 の native current-source stage0 producer/runtime、selfhost/native/MCP parity、Mac Apple Silicon と
Linux x86_64 の実 runtime evidence は未完了であり、TODO の `[~]` を維持する。
