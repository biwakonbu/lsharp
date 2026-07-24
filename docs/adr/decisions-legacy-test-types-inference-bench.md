# ADR: 型推論の限界値 benchmark 入口

- Status: Accepted (verified slice)
- Date: 2026-07-24
- Scope: `LEGACY-TEST-01` / imp-07 D-3

## Context

深い型注釈と巨大レコードの safety contract は追加されたが、occur-check 付近の推論コストを同じ fixture で再計測する入口がなかった。性能値を手作業で比較すると fixture と測定条件が揺れる。

## Decision

`lsharp-types` に production dependency へ影響しない Criterion bench target `infer_limits` を追加する。次の parse → type inference を benchmark する。

- `Box` 128 段の型注釈付き identity
- `Int` フィールド 256 個の `Wide` レコード型注釈付き identity

fixture は benchmark 内で決定的に生成し、sample size は Criterion の CLI で変更できる。初期値は観測用 baseline とし、閾値や線形性の合否をこの ADR では定義しない。

## Evidence

- Gate: `cargo bench -p lsharp-types --bench infer_limits -- --noplot --sample-size 10`
- 2026-07-24 local run: deep 128 は `[1.2223 ms, 1.2291 ms, 1.2359 ms]`、wide 256 は `[11.493 ms, 11.638 ms, 11.862 ms]`
- Build/lint: `cargo bench -p lsharp-types --bench infer_limits --no-run`、`cargo clippy -p lsharp-types --bench infer_limits -- -D warnings`

測定値は実行環境依存の観測値であり、release gate の固定閾値ではない。

## Consequences

- 同一 fixture の occur-check / 深い型推論コストを再現可能に比較できる。
- nightly 4096 cases、性能回帰の閾値・issue 化ルール、GC/rooting/native target の証拠は別タスクとして残る。
