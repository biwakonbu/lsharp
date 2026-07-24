# ADR: 型推論の巨大レコードに対する限界値契約

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `LEGACY-TEST-01` / imp-07 D-3

## Context

深い型注釈の panic guard に加えて、フィールド数が増えたレコード型を parser と型推論が安全に扱えることを確認する必要がある。巨大レコードは、フィールド走査・レコード登録・注釈の unify がそれぞれ別の負荷境界になる。

## Decision

`lsharp-types` の integration contract として、`Int` フィールドを 128 個および 256 個持つ `Wide` レコードを生成し、`Wide` 型注釈付き identity 関数を parse → inference する。各ケースを `catch_unwind` で包み、panic せず、型推論が成功することを固定する。

この契約は巨大レコード構造の safety regression を検出する。criterion による推論時間の上限、GC スロット上限、Wasm runtime の再帰限界は別の証拠であり、この ADR の成功を `LEGACY-TEST-01` 全体の完了へ拡大解釈しない。

## Evidence

- Test: `crates/lsharp-types/tests/infer_limits.rs::wide_record_type_annotations_do_not_panic`
- Fixture widths: 128 / 256 fields
- Gate: `cargo test -p lsharp-types --test infer_limits -- --test-threads=1`

## Consequences

- 型定義登録から注釈の型推論まで、巨大レコードの bounded success path に deterministic な panic guard ができる。
- 性能計測、GC leak/limit、runtime recursion limit、Linux x86_64 native gate は未完了のまま残る。
