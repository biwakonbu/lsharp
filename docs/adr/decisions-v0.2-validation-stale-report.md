# ADR: v0.2 validation report の stale facts

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `lsharp-types::validation::ValidationReport`
- Related: `EC-M2-02` / `EC-M2-03`、`docs/adr/decisions-v0.2-review-stale-propagation.md`、
  `docs/adr/decisions-v0.2-selfhost-validation-stale-report.md`

## Context

`IntentGraph::stale_subjects()` は invalidation の canonical projection を持つが、
`ValidationReport` がその事実を捨てていた。そのため、trace gap がなく独立 review も存在する
graph で stale review/evidence が残っていても `pass` と判定され得た。

## Decision

`ValidationReport` に次の facts を追加する。

- `stale_reviews`: stale と判定された review ID の件数
- `stale_evidence`: stale と判定された evidence ID の件数

`IntentGraph::validate()` は contradiction がある場合は従来通り `fail` を優先し、
contradiction がなく stale facts が一つでもある場合は `unknown` を返す。JSON projection は
`stale_reviews` / `stale_evidence` を required integer fields とし、text projection は
`stale-reviews` / `stale-evidence` を deterministic な末尾行として出力する。

## Evidence

- RED: `cargo test -p lsharp-types --test validation_stale_report -- --nocapture` は
  `ValidationReport::stale_reviews` / `stale_evidence` 未実装で compile error。
- GREEN: 同コマンド 1 passed。
- Regression: `validation_json` 1 passed、`validation_text` 1 passed、`intent_validation`
  6 passed、`validation_schema` 2 passed。

## Boundary

これは Rust canonical validation model と schema/text/JSON projection の ADR である。
selfhost の report parity は別 ADR の Rust-host actual Wasm verified slice として接続する。
native stage0、Mac Apple Silicon / Linux x86_64 artifact/runtime、provider authentication と
外部 review lifecycle の実証は未完了であり、EC-M2 aggregate は TODO の `[~]` を維持する。
