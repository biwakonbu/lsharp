# ADR: regex bounded repeat の property contract

- Status: Accepted (verified slice)
- Date: 2026-07-27
- Scope: `LEGACY-TEST-01` / `imp-07`

## Context

文字列制約の正規表現 matcher には、固定回数・上限付き・上限なしの bounded repeat が
実装されている。既存の例示テストは代表的な数値だけを確認しており、最小値と入力長の
組み合わせ全体で受理範囲を守る契約がなかった。

## Decision

`crates/lsharp-types/tests/regex_bounded_repeat_property.rs` に proptest の bounded lane を
追加する。

- `^a{min,max}$` は生成した閉区間の長さだけを `Satisfied` とする
- `^a{min,}$` は `min` 以上の全長を `Satisfied` とする
- 各 property は 64 cases、入力長は bounded にし、失敗時の source persistence は無効化する
- matcher の実装や公開 API は変更せず、`ConstraintDef::Matches` の既存 observable contract
  を検証する

この slice は `LEGACY-TEST-01` 全体（fuzz、runtime limit、native parity、全 generator）を
完了扱いにせず、TODO の aggregate は未完のまま残す。

## Evidence

```text
cargo test -p lsharp-types --test regex_bounded_repeat_property -- --nocapture
2 passed (各 property 64 cases)
```

pure Rust の制約評価テストであり、Wasm artifact や native stage0 の挙動を変更しないため、
native gate はこの slice の完了条件に含めない。

## Consequences

- bounded repeat の境界回帰を、単一の例示値ではなく生成された長さ区間で検知できる。
- proptest の dev-dependency と integration test に閉じ、配布 artifact の依存関係は変わらない。
- `LEGACY-TEST-01` の残る property 全体、runtime/GC、native target、性能閾値は引き続き未完である。
