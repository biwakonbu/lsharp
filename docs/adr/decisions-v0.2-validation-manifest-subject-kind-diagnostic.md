# ADR: v0.2 validation manifest の subject kind 診断

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: Rust canonical version 1 manifest parser の typed subject error
- Related: `EC-M2-02`、`EC-M2-03`、`EC-M3-01`

## Context

manifest parser は `evaluates` の subject に `contract` / `review`、`invalidates` の subject に
`intent` / `claim` / `contract` が来た場合に fail-closed していたが、欠落 node と同じ
`MissingNodeReference` に分類していた。これは wire kind の不一致と referential closure の欠落を
区別できず、schema/source adapter の typed-kind boundary と診断契約が揃っていなかった。

## Decision

- `ValidationInputError::InvalidSubjectKind` を追加し、relation、kind、stable wire ID を保持する。
- `evaluates.subject` の不正 kind と `invalidates.subject` の不正 kind はこの variant を返す。
- 実在しない intent/claim/evidence/review の ID は従来どおり `MissingNodeReference` または
  `GraphError::Missing*` として扱う。

## Evidence

- RED: `parse_manifest_reports_invalid_evaluates_subject_kind` と
  `parse_manifest_reports_invalid_invalidates_subject_kind` は新しい診断 variant がない状態で失敗した。
- GREEN: 両 fixture が relation、kind、stable ID を保持した `InvalidSubjectKind` で拒否される。
- `cargo test -p lsharp-types --test validation_input`
- `cargo test -p lsharp-types`
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`

## Boundary and follow-up

これは Rust canonical manifest parser の診断分類に限定した verified partial sliceである。source/native
stage0 の diagnostic/exit parity、selfhost/native manifest parser、CLI/MCP、current-source artifact/runtime、
Mac/Linux matrix、EC-M2-02/EC-M3 aggregate は未完了で、TODO の `[~]` を維持する。
