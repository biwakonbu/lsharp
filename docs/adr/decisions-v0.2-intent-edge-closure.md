# ADR: v0.2 canonical IntentGraph の typed edge closure

- Status: Accepted (verified partial slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-types/src/{validation,evidence}.rs`
- Related: `EC-M2-01`, `EC-M2-02`, `docs/development/planning/v0.2-validation-model.md`

## Context

source adapter と version 1 manifest parser は endpoint の存在を個別に検査するが、後続の
selfhost/native producer が直接 `IntentGraph` API を使う場合、その境界を別実装にすると
未登録の Intent / Claim / Assumption を参照する edge が canonical graph に入る。
typed edge の kind と graph-owned node の存在を同じ model 境界で保証する必要がある。

## Decision

`IntentGraph::add_edge` は `EvidenceGraph` へ登録する前に、次の graph-owned endpoint を
fail-closed に検査する。

- `motivates`: Intent と Claim
- `constrained-by`: Claim と Assumption
- `tested-by`: Claim（Contract は外部 executable boundary）
- `supports` / `contradicts`: Claim（Evidence は既存 registry で検査）
- `evaluates`: Intent または Claim の subject（Evidence subject は registry で検査）

未登録 node は `GraphError::MissingNode` とし、失敗した edge は graph に追加しない。
`invalidates` の Change / Review は現行 model で node registry を持たない外部識別子として
扱い、Evidence subject の存在だけを既存の `EvidenceGraph` closure に委ねる。

## Consequences

- direct canonical API、manifest input、source adapter の node endpoint 境界が一致する。
- producer は node を先に登録してから edge を追加する必要がある。
- Contract / Change / Review の registry は別の M2 後続 sliceで設計する。今回の変更だけで
  selfhost/native parity や `validate` 完了を宣言しない。

## Evidence

- `crates/lsharp-types/tests/intent_validation.rs`
- `cargo test -p lsharp-types`
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`
