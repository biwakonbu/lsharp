# ADR: v0.3 review lifecycle の clock 時点 state

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `crates/lsharp-types/src/review_lifecycle.rs`, `crates/lsharp-types/src/review_attestation.rs`
- Related: `EC-M3-01`, `EC-M3-02`

## Context

`verify_against_at` は明示 clock を受け取るが、lifecycle registry の最新 event をそのまま
現在 state として使っていた。そのため、snapshot に将来の `active` / `revoked` event が含まれると、
その `effective_at` より前の clock でも transition を適用してしまう。これは attestation の expiry
window と lifecycle の時系列を別々に扱い、future state を verified fact として投影する境界漏れである。

## Decision

- lifecycle registry に `event_at(review_id, at)` を追加し、指定時刻以前の最新 event を返す。
- `verify_against_at` は strict canonical UTC clock を検証したうえで `event_at` を使う。
- 明示 clock を持たない既存 API (`verify_with_lifecycle` / `verify_against`) は snapshot の current
  event semantics を維持し、暗黙の system clock を導入しない。
- `effective_at` は既存の単調性検査と同じ固定長 canonical UTC の文字列順で比較する。

## Evidence

- RED: `lifecycle_transition_is_not_effective_before_its_clock` が future `active` event を含む
  snapshot に対し、effective 前も `verified` になってしまう現状を検出した。
- GREEN: 同テストで effective 前は `unverified`、effective 時点は `verified` を確認した。
- Regression: `cargo test -p lsharp-types --test review_signature` と lifecycle focused gate を通過。

## Boundary

これは Rust canonical verifier の lifecycle time selection に限定した verified partial slice である。
selfhost/native producer parity、provider snapshot acquisition、CLI/MCP report projection、Mac/Linux
artifact/runtime、EC-M3 aggregate completion は未完了のため `TODO.md` の partial state を維持する。
