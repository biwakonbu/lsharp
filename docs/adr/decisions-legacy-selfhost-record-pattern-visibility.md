# ADR: Selfhost record pattern schema visibility contract

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: `selfhost/src/Types/TypeInferPattern.ls`,
  `crates/lsharp-wasm/tests/e2e/selfhost_typeinfer_quote_patterns.rs`
- Related: `LEGACY-LANG-01`,
  `docs/adr/decisions-legacy-exec-wasmgc-record-pattern.md`

## Context

`TypeInferPattern.ls` は全 program の record schema を `record-env` に保持する一方、
module 境界の visibility は現在の value environment に登録された constructor scheme で
判定する。以前の低レベル E2E fixtureは schema registryだけを用意して value environmentを
空にしていたため、private recordを拒否する現在の実装を失敗として観測していた。

## Decision

- canonical record patternの型推論は、record schemaだけでなく現在の環境から取得した
  constructor schemeを必要とする。
- 可視 constructorが存在する場合はその戻り型をschema field型へ接続し、child binderを
  declared field typeへunifyする。
- `record-env`にschemaがあるだけで、value environmentにconstructorがない場合は
  `error-code-undefined`でfail-closedにする。これによりprivate recordのmodule漏洩を防ぐ。
- E2E fixtureは可視成功と非可視拒否を同じschema registryで同時に検証する。

## Evidence

- RED: 既存 fixtureを current visibility contractのまま実行すると、空のvalue environmentで
  `result-failed=1`となり、期待値 `0`との差分で失敗した。
- GREEN: `test_e2e_selfhost_typeinfer_record_pattern_uses_declared_field_type` は可視
  constructor schemeを追加後、field binderが `Int` (`tag=1`, `hash=100`)となり、非可視経路は
  `result-failed=1`を返すことを確認した。
- 回帰: `selfhost_typeinfer_quote_patterns` 13 tests、
  `selfhost_typeinfer_private_visibility` 1 testが passした。
- selfhost sourceは変更していないため、このtest-contract修正ではnative stage0を再生成せず、
  直近のcurrent-source Linux fixed-point evidenceを引き続き参照する。

## Boundary

これは selfhost type-inference の record schema visibility と field binding の verified
partial sliceである。record pattern全体のsemantic parity、import/parametric/deep cases、
runtime/ftable/linear-memory ABI、Mac Apple SiliconとLinux x86_64の変更後artifact matrix、
`LEGACY-LANG-01` aggregateの完了は意味しない。未完了境界は `TODO.md` の `[~]` を維持する。
