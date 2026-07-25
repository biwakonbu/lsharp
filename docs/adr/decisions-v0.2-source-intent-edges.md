# ADR: v0.2 source intent edge syntax と Rust adapter

- Status: Accepted (partial slice)
- Date: 2026-07-25
- Scope: EC-M2 source node-to-node edges

## Context

source の stable node は `IntentGraph` へ投影できるようになったが、宣言 metadata から
typed edge を作る境界がなく、ID を raw string のまま解釈する余地が残っていた。edge は
endpoint の node kind と project-level existence を確認しなければ、source の typo や
宣言順依存を `validate` の入力へ持ち込む。

## Decision

source metadata に endpoint の wire ID を二つ持つ、次の node-to-node edge form を追加する。

```lisp
:motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
:constrained-by "claim:checkout/cancel-rejects-shipped" "assumption:checkout/state-authoritative"
```

parser は form の source order と directive span を保持する。`validation_source` はまず全宣言の
node registry を構築し、その後 edge を走査する二段階 projection とする。これにより edge が
node 宣言より先に現れても解決できる。各 endpoint は `IntentId` / `ClaimId` / `AssumptionId`
として parse し、kind mismatch、不正 wire、存在しない node は fail-closed で拒否する。
存在しない node の診断には、その edge directive の source span を含め、入力位置を失わない。

`tested-by` は Contract node と legacy contract suite の ID 解決、`supports` / `contradicts`
は evidence record の生成を必要とするため、この slice では parser/adapter に追加しない。
manifest の既存 typed edge model は変更せず、source adapter が構築した edge をそのまま
既存 JSON projection へ渡す。

## Consequences

- source から Intent→Claim と Claim→Assumption の typed edge を宣言順に登録できる。
- node の宣言順に依存せず、endpoint kind と存在を一つの Rust adapter で検査できる。
- contract/evidence を必要とする edge、source manifest の完全生成、selfhost/native parity は
  未接続のまま明示される。source graph だけで `validate` の pass は宣言しない。
- 既存 contract inventory は新しい edge metadata を presentation/graph form として無視し、
  legacy contract の互換 projection を変えない。

## Evidence

- `crates/lsharp-syntax/tests/intent_edges.rs`
- `crates/lsharp-types/tests/validation_source.rs`
- `source_adapter_reports_orphan_edge_with_directive_span`
- `cargo test -p lsharp-syntax --test intent_edges`
- `cargo test -p lsharp-types --test validation_source`
