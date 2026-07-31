# ADR: v0.3 review lifecycle の effective time 単調性

- Status: Accepted (verified partial slice)
- Date: 2026-07-31
- Scope: `crates/lsharp-types/src/review_lifecycle.rs`
- Related: `EC-M3-02`

## Context

Lifecycle event は review ID ごとに sequence 順へ reduce される。既存実装は sequence の重複・
逆行と state transition を拒否していたが、後続 event の `effective_at` が前の event より過去でも
受理していた。これは同じ sequence ordering から過去の lifecycle state へ戻る入力を許し、provider
snapshot の順序を deterministic にしても時系列の意味を壊す。

## Decision

- 同じ review ID の既存 event より `effective_at` が早い event を `EffectiveTimeRollback` として拒否する。
- timestamp は event construction 時に strict UTC の固定長形式へ検証済みなので、canonical timestamp
  の文字列順を chronological order として比較する。
- sequence rollback、duplicate sequence、state transition の既存 error precedence は変更しない。
- lifecycle provider の取得、clock、attestation signature の検証はこの slice の責務外とする。

## Evidence

- RED: `lifecycle_rejects_effective_time_rollback` が、sequence 2 の event を sequence 1 より過去の
  `effective_at` で追加したときの未定義 error variant を固定した。
- GREEN: 同テストが `LifecycleError::EffectiveTimeRollback` を確認して pass。
- Regression: `cargo test -p lsharp-types --test review_lifecycle` は 6件、
  `cargo test -p lsharp-types --test review_signature` は 12件 pass。

## Boundary

これは Rust canonical lifecycle reducer の時系列入力境界だけを閉じる verified partial slice である。
selfhost/native producer parity、provider snapshot、report/MCP projection、Mac/Linux artifact/runtime、
EC-M3-02 aggregate completion は未完了のため `TODO.md` の `[~]` を維持する。
