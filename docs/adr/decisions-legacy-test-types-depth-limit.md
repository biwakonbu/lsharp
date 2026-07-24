# ADR: 型推論の深い型注釈に対する限界値契約

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `LEGACY-TEST-01` / imp-07 D-3

## Context

`LEGACY-TEST-01` は、型推論の深い構造を入力したときに stack overflow や未捕捉 panic を起こさない限界値テストを要求している。既存の occurs-check 契約は自己適用という失敗境界を固定しているが、深くネストした型注釈を通る成功経路の証拠はなかった。

## Decision

`lsharp-types` の integration contract として、`Box` を 32 / 64 / 128 段ネストした型注釈を持つ identity 関数を parse → inference する。各ケースを `catch_unwind` で包み、panic せず、型推論が成功することを固定する。

この契約は、深い型構造の safety regression を検出する。occur-check の性能計測、GC の限界値、Wasm 実行時の再帰 stack 限界は別の証拠として扱い、この ADR の成功を `LEGACY-TEST-01` 全体の完了へ拡大解釈しない。

## Evidence

- Test: `crates/lsharp-types/tests/infer_limits.rs::deeply_nested_type_annotations_do_not_panic`
- Fixture depths: 32 / 64 / 128 nested `Box` applications
- Gate: `cargo test -p lsharp-types --test infer_limits -- --test-threads=1`

## Consequences

- 深い型注釈の parser → type inference 成功経路に deterministic な panic guard ができる。
- 型推論の criterion 性能計測、巨大レコード、GC leak/limit、runtime recursion limit は未完了のまま残る。
