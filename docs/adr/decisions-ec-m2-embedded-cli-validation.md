# ADR: EC-M2 EmbeddedCli validation source/report/exit boundary

- Status: Accepted (verified slice)
- Date: 2026-07-27
- Scope: `selfhost/src/App/EmbeddedCli.ls` と EmbeddedCli runtime bundle

## Context

`App.Cli` には `validate --source <file> --format json [--emit-manifest <path>]` の
selfhost source validation がある一方、`EmbeddedCli` は command routing、validation
module import、runtime bundle のいずれも未接続だった。さらに `run-check-program` は
`context`、`program`、`analysis` の root を積んだまま二つの return path に到達し、
EmbeddedCli の実行時に `ImbalancedExit` を起こしていた。

## Decision

- `EmbeddedCli` に `validate` command と `--source` / `--format json` option parser を追加し、
  source parser → intent/evidence graph adapter → fact-oriented JSON report → exit code の
  順に接続する。
- report は `status`、`trace_gaps`、`open_questions`、`independent_reviews`、
  `contradicting_observations` のみを返し、`verified` shortcut を追加しない。
  `unknown` は exit `2`、contradiction は exit `1`、入力/parse/write failure は exit `1`
  として fail-closed にする。
- EmbeddedCli runtime bundle に `IntentSource.ls` と `Evidence.ls` を含める。
- modular compile でも validation adapter の vector constructor が解決できるよう、
  `IntentSource` / `Evidence` から `Syntax.Parser`（および `Evidence` から `Syntax.AST`）を
  明示 import する。
- `run-check-program` の両 return path で root を LIFO に三回 pop し、root lifetime を
  balance する。

この ADR は source/report/exit の verified slice だけを扱う。version 1 manifest parser/
producer parity、manifest emission の artifact boundary、MCP、native stage0、Mac/Linux
two-target evidence は後続タスクとして残す。

## Evidence

1. RED: `test_e2e_selfhost_embedded_cli_main_with_args_validate_source_json_trace_gap` を
   先に追加し、初回実行で既存 `RootLifetime::ImbalancedExit`（depth 3）を観測した。
2. GREEN: root cleanup、command wiring、bundle import を実装し、同じ E2E を 1 pass
   （251.15 秒）で確認した。未接続 intent が `unknown`、trace-gap code/subject、exit `2`
   として出力され、`verified` が存在しないことを固定した。

## Consequences

EmbeddedCli の source validation は `App.Cli` と同じ report/exit boundary を持つ verified
slice になった。一方、EC-M2-03 の aggregate completion 条件は満たさないため、`TODO.md`
の `[~]` を維持し、manifest/native/MCP/two-target evidence を次の RED とする。
