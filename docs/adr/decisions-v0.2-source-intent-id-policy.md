# ADR: source intent node の stable ID 省略ポリシー

- Status: Accepted (partial slice)
- Date: 2026-07-25
- Scope: EC-M2-01 source node identity

## Context

intent、claim、assumption、open-question は Rust/selfhost と対応 target の間で同じ
evidence subject identity を共有する必要がある。source span、宣言順、formatter 出力、
Rust hash から ID を補うと、宣言の移動・整形・実装 target の違いで identity が変わる。

## Decision

source node の stable ID は常に明示 wire value として受け取る。ID を省略した form に対する
自動命名、関数名・module 名からの推測、span/order/hash による fallback は導入しない。

```lisp
:intent "intent:namespace/key" "purpose"
:claim "claim:namespace/key" "observable claim"
:assumption "assumption:namespace/key" "required premise"
:open-question "open-question:namespace/key" "unresolved question"
```

必要な ID がない source は parser の `LS0101` 入力診断で fail-closed に拒否する。source
adapter も ID を補完せず、wire kind、重複、本文を canonical model の検査へ渡す。

## Consequences

- source を整形・移動しても、明示された namespace/key が変わらない限り evidence subject は変わらない。
- 新しい source node には利用者が stable ID を設計する必要があり、後から自動命名へ暗黙に移行しない。
- source manifest emission と selfhost/native parser/adapter parity は別の残件として扱う。

## Evidence

- `crates/lsharp-syntax/tests/intent_metadata.rs`
- `crates/lsharp-types/tests/validation_source.rs`
- `cargo test -p lsharp-syntax --test intent_metadata`
- `cargo test -p lsharp-types --test validation_source`
