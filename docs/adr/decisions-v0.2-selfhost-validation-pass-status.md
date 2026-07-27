# ADR: v0.2 selfhost Cli validation の pass status parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `selfhost/src/App/Cli.ls`、`validate --source` の status/exit projection
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-selfhost-validation-text-report.md`

## Context

Rust canonical `IntentGraph::validate()` は contradiction を `fail`、trace gap・open question・
独立 review 不足・stale fact を `unknown`、それ以外を `pass` とする。EmbeddedCli はこの順序を
既に実装していたが、App.Cli は contradiction 以外を常に `unknown` / exit `2` としていたため、
complete source graph が Rust の report/exit contract と一致しなかった。

## Decision

- App.Cli に trace-gap count と Rust canonical status rule を実装する。
- JSON と text は同じ status code projection を再利用し、complete graph は `status: pass` / exit `0`、
  contradiction は `fail` / exit `1`、未完了 graph は `unknown` / exit `2` とする。
- contradiction を先に判定し、stale facts、trace gap、open question、独立 review 0 を順に
  unknown boundary として扱う。`verified` shortcut は追加しない。

## Evidence

- RED: `cargo test -p lsharp-wasm --test e2e selfhost_cli_validate_source_text_reports_pass -- --nocapture`
  は現行 App.Cli の exit `2`（期待 `0`）で失敗した。
- 実装初回 RED: status rule の多引数 `or` は L# 二項 operator 契約に反し、selfhost bundle の
  `ArgMismatch` を返した。nested-if へ修正した。
- GREEN: 同 focused test は `1 passed`。Cli complete graph の deterministic text report と exit `0`
  を確認した。
- Regression: `cargo test -p lsharp-wasm --test e2e selfhost_cli_validate_source_json_reports -- --nocapture`
  は trace-gap、contradiction、stale の `3 passed`。
- Source contract: `cargo test -p lsharp-syntax --test selfhost_cli_validation_contract -- --nocapture`
  は `1 passed`。

## Boundary

Rust-host actual Wasm の App.Cli source/report/exit parity までを verified slice とする。EmbeddedCli
との status rule の共通化、native stage0 producer/runtime parity、native MCP、両 supported target の
artifact/runtime evidence、外部 provenance lifecycle は未完了であり、TODO の `EC-M2-03` `[~]` を維持する。
