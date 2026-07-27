# ADR: v0.2 selfhost validation report の stale parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `selfhost/src/Tools/Validation/Stale.ls`、`App.Cli`、`App.EmbeddedCli`
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-validation-stale-report.md`

## Context

Rust canonical `ValidationReport` は stale review/evidence を JSON/text facts として返すが、
selfhost の source validation report はその facts を捨て、invalidated review を含む complete
graph を pass として返し得た。Cli と EmbeddedCli が別々に stale 判定を持つと declaration order
と重複除去も diverge する。

## Decision

- `Tools.Validation.Stale` に Rust `IntentGraph::stale_subjects()` と同じ projection を置く。
- registry の `outcome=stale`、宣言順の `invalidates`、stale review の `evaluates` evidence を
  review/evidence ID の first-seen order で重複除去する。
- `App.Cli` と `App.EmbeddedCli` は同じ metrics を report に追加し、`stale_reviews` /
  `stale_evidence` を JSON facts として出力する。
- EmbeddedCli は contradiction を優先し、stale facts が残る場合は `unknown` / exit `2` とする。
  Cli も同じ facts を出力し、既存の source validation unknown boundary を維持する。

## Evidence

- RED: `cargo test -p lsharp-wasm --test e2e stale_review_and_evidence -- --nocapture` は
  既存 selfhost report が `pass` または stale fields 欠落となり、2 tests failed。
- GREEN: 同コマンドの再実行は `2 passed`。core の既存 trace-gap / contradiction / stale
  report 回帰は `cargo test -p lsharp-wasm --test e2e selfhost_cli_core::test_e2e_selfhost_cli_validate_source_json_reports -- --nocapture`
  で `3 passed`。
- Source module contract: `cargo test -p lsharp-syntax --test selfhost_cli_validation_contract -- --nocapture`
  は `1 passed`。

## Boundary

Rust-host actual Wasm の `App.Cli` / `App.EmbeddedCli` source/report/exit parity までを verified
slice とする。native stage0 producer/runtime parity、native MCP、atomic/durable native writer、
両 supported target の artifact/runtime evidence、provider authentication と外部 review lifecycle
は未完了であり、TODO の `[~]` を維持する。
