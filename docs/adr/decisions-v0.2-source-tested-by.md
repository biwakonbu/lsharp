# ADR: source `:tested-by` edge の graph adapter

- Status: Accepted (partial)
- Date: 2026-07-25
- Scope: EC-M2-01 / EC-M2-02 / EC-M2-03

## Context

source の `:intent` / `:claim` と `:motivates` / `:constrained-by` は Rust の
`IntentGraph` へ投影できるようになったが、claim と executable contract の関係だけが
source input から欠落していた。そのため、source に contract test の対応があっても
`validate --source` は claim trace gap を報告し続けた。

## Decision

次の metadata form を source の宣言位置で受理する。

```lisp
:tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
```

- parser は claim ID と contract ID を lossless に保持し、directive span と source order を維持する。
- source adapter は全 node を先に登録した後、`ClaimId` / `ContractId` を parse して
  `Edge::TestedBy` を追加する。
- claim endpoint の typed kind mismatch と orphan reference は fail-closed に拒否する。
- Contract の executable definition と evidence record はこの edge から暗黙生成しない。
  `validate --source` は tested-by edge で claim trace gap を閉じるが、evidence record が
  未接続なら status は `unknown` のままにする。
- `supports` / `contradicts`、manifest emission、selfhost/native parity は後続境界とする。

## Evidence

- `cargo test -p lsharp-syntax --test intent_edges`
- `cargo test -p lsharp-types --test validation_source`
- `cargo test -p lsharp-driver --test validate_cli`

## Remaining boundary

source から Contract の canonical definition、実行結果を持つ `Evidence`、
`supports` / `contradicts` edge を生成する入力契約は未実装である。これらを追加するまで、
source validation の `unknown` を `pass` へ拡張しない。
