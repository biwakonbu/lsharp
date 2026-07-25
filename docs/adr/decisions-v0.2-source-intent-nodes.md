# ADR: v0.2 source intent node syntax と Rust adapter

- Status: Accepted (partial slice)
- Date: 2026-07-25
- Scope: EC-M2-01 source node registry

## Context

M2 の stable ID model と JSON manifest parser は Rust 側で固定済みだったが、L# source から
`IntentNode` を作る入力境界がなく、source metadata を自由文字列として解釈する余地があった。
ID を span、宣言順、Rust hash、formatter 出力から暗黙に導出すると、Rust/selfhost や整形前後で
evidence subject が変わる。

## Decision

宣言 metadata に次の 4 form を追加し、wire ID と本文を必須にする。

```lisp
:intent "intent:namespace/key" "purpose"
:claim "claim:namespace/key" "observable claim"
:assumption "assumption:namespace/key" "required premise"
:open-question "open-question:namespace/key" "unresolved question"
```

parser は source order と directive span を `MetadataForm` に保持する。`validation_source` adapter
は form kind と wire prefix を照合し、`IntentNode` を `IntentGraph` の node registry へ登録する。
duplicate ID、空本文、不正 wire、kind mismatch は fail-closed にする。既存の `:doc`、`:rationale`、
contract forms は別の metadata/contract projection として扱い、この adapter が自由に intent や
evidence へ変換しない。

## Consequences

- source node identity が JSON manifest と同じ stable ID contract を使える。
- nested module、private、impl の宣言を同じ adapter で走査できる。
- source edge/evidence、ID 省略の命名規則、manifest emission、selfhost/native parity は未接続の
  まま明示される。node registry だけで `validate` の pass を宣言しない。

## Evidence

- `crates/lsharp-syntax/tests/intent_metadata.rs`
- `crates/lsharp-types/tests/validation_source.rs`
- `cargo test -p lsharp-syntax --test intent_metadata`
- `cargo test -p lsharp-types --test validation_source`
