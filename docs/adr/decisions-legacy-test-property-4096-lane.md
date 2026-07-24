# ADR: property test 4096-case manual lane

- Status: Accepted (verified slice)
- Date: 2026-07-25
- Scope: `LEGACY-TEST-01` / imp-07 D-4

## Context

`lsharp-syntax` と `lsharp-types` の property test は、通常の crate test では 64 cases に抑えている。通常の開発を遅くせずに generator の panic safety、AST roundtrip、unification symmetry、bounded expression inference を広い入力で再検証する、手動/nightly 用の再現可能な入口が必要だった。

## Decision

[`scripts/ci/test-property-nightly.sh`](../../scripts/ci/test-property-nightly.sh) を、property test の広いケース数を実行するローカル手動/nightly lane とする。引数なしの既定値は次の通り固定する。

- `PROPTEST_CASES=4096`
- `PROPTEST_RNG_SEED=20260725`
- `--test-threads=1`

対象は次の 4 property test とし、各テストを個別の `cargo test` として直列実行する。

- `lsharp-syntax::property_tests::parser_never_panics_for_bounded_arbitrary_bytes`
- `lsharp-syntax::roundtrip_property_tests::pretty_printed_ast_reparses_to_the_same_source`
- `lsharp-types::infer::unify_property_tests::unify_success_is_symmetric`
- `lsharp-types::infer::inference_property_tests::bounded_expression_inference_never_panics`

`PROPTEST_CASES` / `PROPTEST_RNG_SEED` は識別実験のために環境変数で上書きできる。`--dry-run` は実行せず、解決した profile と command だけを出力する。GitHub Actions の新規 workflow は追加せず、ローカル実行を正本とする。

## Evidence

- Contract: `bash scripts/ci/test-property-4096-contract.sh` → `property 4096-case lane contract passed`
- Wide lane: `scripts/ci/test-property-nightly.sh` → 4 tests passed
  - parser panic safety: 4096 cases, 0.15 s
  - AST roundtrip: 4096 cases, 0.52 s
  - unification symmetry: 4096 cases, 0.46 s
  - bounded expression inference: 4096 cases, 0.93 s
- Default focused regression: `cargo test -p lsharp-syntax --lib parser_never_panics_for_bounded_arbitrary_bytes -- --test-threads=1`、`cargo test -p lsharp-types --lib unify_success_is_symmetric -- --test-threads=1` → 各 1 passed
- Script syntax: `bash -n scripts/ci/test-property-nightly.sh scripts/ci/test-property-4096-contract.sh`

## Consequences

- 通常の 64-case property test の開発速度と、4096-case の広い safety 検証を分離できる。
- 固定 seed により失敗を同じ profile で再現しやすい。別 seed の探索や CI スケジュールへの接続は別タスクとする。
- `LEGACY-TEST-01` aggregate の GC slot 32768、runtime `memory.grow` 上限、rooting stress/static lint、native stage0、性能回帰閾値、full fuzz target は未完了のまま残る。
