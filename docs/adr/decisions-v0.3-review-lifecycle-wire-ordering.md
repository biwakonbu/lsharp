# ADR: v0.3 review lifecycle wire の宣言順非依存化

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `crates/lsharp-types` の review lifecycle registry / version 1 wire parser
- Related: [`v0.3-review-provenance-lifecycle.md`](../development/planning/v0.3-review-provenance-lifecycle.md)、`EC-M3-02`

## Context

v0.3 の lifecycle snapshot は declaration order ではなく `(review_id, sequence)` の順で
deterministic に reducer へ渡す設計である。しかし既存の `ReviewLifecycleRegistry::add_event`
は append-only API であり、wire parser が JSON 配列をそのまま渡していたため、sequence 2 の
event が sequence 1 より先に現れるだけで `InvalidInitialState` / rollback になっていた。

## Decision

- `ReviewLifecycleRegistry::from_events` を追加し、入力全体を review ID、sequence の順に sort
  してから既存の append-only reducer へ渡す。
- `add_event` の strict な duplicate、rollback、invalid transition 検査は維持する。
- `parse_review_wire` は lifecycle events を変換して `from_events` に渡す。したがって wire の
  配列順は意味を持たず、出力の lifecycle order は常に deterministic になる。
- 同一 review ID・sequence の duplicate、terminal state からの resurrection、invalid state は
  sort 後も従来どおり fail-closed に拒否する。

## Evidence

- RED: sequence 2 `revoked` を sequence 1 `active` より先に置いた wire fixture が、既存 parser
  で `InvalidInitialState` になった。
- GREEN: 同じ fixture が parse でき、reduced state が `revoked`、events の sequence が `[1, 2]`
  になる focused test を追加した。
- `cargo test -p lsharp-types`: 221 unit tests と全 integration tests が通過。
- review lifecycle / attestation / signature / wire focused tests: 26 tests が通過。
- `cargo clippy -p lsharp-types --tests -- -D warnings` と変更対象 wire/test の rustfmt check が通過。

## Boundary

これは Rust canonical wire/reducer の verified partial slice である。selfhost/native stage0 の
lifecycle parser/runtime parity、Mac Apple Silicon / Linux x86_64 artifact evidence、provider
snapshot 取得、trust/lifecycle の release policy は未完了であり、`TODO.md` の `EC-M3-02` は
`[~]` のまま維持する。
