# ADR: v0.3 review lifecycle の strict effective_at

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `lsharp-types` の lifecycle event constructor / review wire における `effective_at`
- Related: [`decisions-v0.3-review-lifecycle.md`](decisions-v0.3-review-lifecycle.md)、
  [`decisions-v0.3-review-attestation-expiry-clock.md`](decisions-v0.3-review-attestation-expiry-clock.md)、
  `EC-M3-02`

## Context

既存の lifecycle reducer は `effective_at` が空でないことだけを検査していた。そのため、
`not-a-timestamp` や存在しない日付を含む event が registry に入り、同じ snapshot でも
Rust と selfhost/native の parser が異なる時刻意味論を持ち得た。attestation の
`issued_at` / `expires_at` と同じ deterministic な UTC 境界が lifecycle event にも必要である。

## Decision

- `ReviewLifecycleEvent::new` は `effective_at` を `YYYY-MM-DDTHH:MM:SSZ` の strict UTC 形式、
  実在 Gregorian 日付、秒 `00..59` として検証する。
- attestation と lifecycle が同じ canonical timestamp parser を共有し、offset、fractional
  seconds、leap second、存在しない日付を `LifecycleError::InvalidTimestamp` で fail-closed にする。
- JSON wire の lifecycle event は constructor を経由するため、malformed `effective_at` は
  registry/review input に投影せず `ReviewWireError::Lifecycle` として停止する。
- event の順序・state transition・provider snapshot の取得責務は既存 lifecycle ADR の境界を維持する。

## Evidence

- RED: malformed `effective_at` fixture は実装前に registry へ受理されることを確認した。
- GREEN: `review_lifecycle` 5 tests と `review_wire` 4 tests で malformed shape / calendar date の
  field/value 付き拒否と既存 transition/roundtrip を固定した。
- 変更 Rust の diff check と focused test を通過した。selfhost/native target parity、両 target の
  runtime/release evidence はこの slice の証拠に含めない。

## Boundary

これは Rust canonical lifecycle timestamp の verified partial slice である。lifecycle の
selfhost/native parser parity、CLI/MCP stage0 wiring、provider snapshot/release provenance、
Mac Apple Silicon / Linux x86_64 runtime gate は未完了であり、`EC-M3-02` は `[~]` のまま維持する。
