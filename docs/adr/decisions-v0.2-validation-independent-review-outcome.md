# ADR: v0.2 validation の独立 review outcome gate

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `lsharp-types::validation::IntentGraph::validate`
- Related: `EC-M2-02`、`EC-M2-03`、`docs/development/planning/v0.2-validation-model.md`

## Context

`ValidationReport::independent_reviews` は review method と
`IndependentReview` independence だけを見ていた。そのため、`outcome=fail` の review evidence
でも独立 review が存在したと数えられ、trace gap のない graph が `status=pass` になり得た。
これは「review がある」ことと「review が明示的に成功した」ことを混同し、失敗した review を
intent validation の完了条件として扱う境界を壊す。

## Decision

- 独立 review gate を満たす evidence は、`method=review`、`outcome=pass`、
  `independence=independent-review` の3条件をすべて満たすものに限定する。
- `fail`、`unknown`、`stale`、`contradicted` の review evidence は
  `independent_reviews` に含めず、他の未完了条件がなければ `status=unknown` へ留める。
- contradiction の `status=fail` 優先順位と、report の JSON/text wire shape は変更しない。
- source/selfhost/native/MCP の parity や review provider の署名認証はこの canonical Rust sliceの
  後続境界として TODO に残す。

## Evidence

- RED: `failed_independent_review_does_not_satisfy_review_gate` は、失敗 review を含む complete
  graph が従来 `pass` / `independent_reviews=1` となることを確認して失敗した。
- GREEN: `IntentGraph::validate()` が passing review のみを数えるよう変更し、同テストが
  `unknown` / `independent_reviews=0` となることを固定した。
- `CARGO_TARGET_DIR=.../target cargo test -p lsharp-types --test intent_validation -- --nocapture`
  は 7 tests passed。

## Boundary and follow-up

これは Rust canonical validation report の outcome gate に限定した verified partial sliceである。
review lifecycle/provider authentication、selfhost/native stage0 producer/runtime、MCP/公開 CLI の
同一 outcome parity、Mac Apple Silicon / Linux x86_64 runtime evidence、EC-M2 aggregate は未完了であり、
TODO の `[~]` を維持する。
