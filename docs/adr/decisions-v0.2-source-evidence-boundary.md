# ADR: source `:supports` / `:contradicts` の evidence registry 境界

- Status: Accepted (partial)
- Date: 2026-07-25
- Scope: EC-M2-02 / EC-M2-03

## Context

Evidence graph の canonical model には Observation と Claim を結ぶ `supports` /
`contradicts` edge がある。一方、source adapter には evidence record を登録する入力境界が
まだなく、source metadata をそのまま無視すると typo や未登録 evidence が validation から
消えてしまう。実体のない edge を追加して `pass` や `unknown` の判定を変えることも避ける必要がある。

## Decision

source metadata では次の form を受理し、wire ID と span を lossless に保持する。

```lisp
:supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
:contradicts "evidence:checkout/cancel-counterexample" "claim:checkout/cancel-rejects-shipped"
```

- parser は observation と claim の raw wire ID を `MetadataFormKind` の専用 variant として保持する。
- source adapter は `EvidenceId` / `ClaimId` を parse し、Claim endpoint の kind と registry 上の存在を
  検査する。
- evidence registry が未接続の間は `SourceGraphError::EvidenceRegistryRequired` を返す。
  この段階では edge を `IntentGraph` に追加せず、未登録 evidence を黙って無視しない。
- `validate --source` はこの境界を入力エラーとして返す。evidence record が存在しない通常の
  node/`tested-by` source は従来どおり `unknown` になり、`pass` を補完しない。
- evidence record の source registry、実 edge 投入、manifest emission、selfhost/native parity は
  後続 task とする。

## Consequences

- source に書かれた `supports` / `contradicts` が parser の unknown metadata として消えることを防ぐ。
- ID と Claim endpoint の typo は evidence registry の実装前でも fail-closed に診断できる。
- source から evidence record を登録できるまでは、これらの edge を含む source は validation report
  を返さず、明示的な入力エラーになる。

## Evidence

- `crates/lsharp-syntax/tests/intent_edges.rs`
- `crates/lsharp-types/tests/validation_source.rs`
- `crates/lsharp-driver/tests/validate_cli.rs`
- `cargo test -p lsharp-syntax --test intent_edges evidence_edges`
- `cargo test -p lsharp-types --test validation_source source_adapter_rejects_evidence_edges`
- `cargo test -p lsharp-driver --test validate_cli validate_source`
