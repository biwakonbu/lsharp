# ADR: v0.2 selfhost validation の deterministic text report

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `selfhost/src/App/Cli.ls`、`selfhost/src/App/EmbeddedCli.ls`、`validate --source`
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-validation-stale-report.md`

## Context

Rust canonical `ValidationReport::to_text()` は JSON と同じ事実を固定順で newline-delimited に
出力する。一方、selfhost の `validate --source` は `--format json` だけを受理し、text を要求する
consumer が未接続だった。Cli と EmbeddedCli が別の projection を持つと、trace gap の順序、件数、
exit code が surface ごとに分岐する。

## Decision

- `--format text|json` を両 selfhost surface の source validation option として受理する。
- text は `status`、trace gap（`code: subject_id`）、`open-questions`、
  `independent-reviews`、`contradicting-observations`、`stale-reviews`、`stale-evidence` を
  Rust `ValidationReport::to_text()` と同じ順序で出力する。
- JSON の wire shape と text の line shape のどちらにも `verified` shortcut を追加しない。
- text/json ともに `pass=0`、`fail=1`、`unknown=2` の source validation exit contract を維持し、
  unsupported format は report を出さず option error (`1`) とする。

## Evidence

- RED: `cargo test -p lsharp-wasm --test e2e selfhost_cli_validate_source_text_reports_trace_gap -- --nocapture`
  は未対応 format のため exit `1`（期待 `2`）で失敗した。
- GREEN: 同 test は `1 passed`。Cli の trace-gap fixture で deterministic text、exit `2`、JSON/
  `verified` 混入なしを確認した。
- GREEN: `cargo test -p lsharp-wasm --test e2e selfhost_embedded_cli_main_with_args_validate_source_text_trace_gap -- --nocapture`
  は `1 passed`。EmbeddedCli の intent-gap fixture でも同じ text/exit boundary を確認した。
- Source contract: `cargo test -p lsharp-syntax --test selfhost_cli_validation_contract -- --nocapture`
  は `1 passed`。

## Boundary

Rust-host actual Wasm の `App.Cli` / `App.EmbeddedCli` source/report/exit parity までを verified
slice とする。native stage0 producer/runtime parity、native MCP、atomic/durable native writer、
両 supported target の artifact/runtime evidence は未完了であり、TODO の `EC-M2-03` `[~]` を維持する。
