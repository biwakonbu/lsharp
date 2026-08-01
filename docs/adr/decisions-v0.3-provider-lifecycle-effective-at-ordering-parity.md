# ADR: provider lifecycle effective-at ordering parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/verify-native-release-identity.py` の review-lifecycle semantic preflight

## Context

native provider preflight は `effective_at` の strict UTC/calendar shapeを検証していたが、同じ
review の sequence が進んだ eventで timestamp が過去へ戻ることを拒否していなかった。Rust
canonical reducer は sequence 順の `effective_at` rollback を `EffectiveTimeRollback` として拒否するため、
同じ lifecycle semantic boundaryに native gapがあった。

## Decision

同じ `review_id` の sequenced recordsについて、直前 eventに `effective_at` がある場合、次の
`effective_at` は lexical UTC orderでそれ以前であってはならない。rollback は
`review lifecycle effective_at rollback` として fail-closed に拒否する。既存の optional timestamp
compatibility、strict shape、sequence、state transition、snapshot digest は維持する。

## Evidence

- RED: `proposed sequence: 1` at `2026-08-02T00:00:00Z` に続く `active sequence: 2` at
  `2026-08-01T23:59:59Z` が native verifier exit `0` になった。
- GREEN: 同じ fixtureを `review lifecycle effective_at rollback` で拒否する test と Rust
  `lifecycle_rejects_effective_time_rollback` の対応を確認した。

## Boundary

これは native/Rust lifecycle effective-at ordering parity に限る verified partial sliceである。live
provider API/auth取得・署名/実 semantic verification、current-source Linux runtime、両 target
packaged/rollback parityは未検証であり、M3-04-N1 / M3-05-N2 / M3-05-N7 / M3-05-N9 は `[~]` のまま維持する。
