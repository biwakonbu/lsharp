# ADR: v0.2 validation manifest の duplicate top-level key

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 manifest envelope
- Related: `EC-M2-03`, `docs/adr/decisions-v0.2-validation-input-parser.md`,
  `docs/adr/decisions-v0.2-validation-input-required-fields.md`

## Context

JSON object の同じ top-level field が複数回現れると、値の上書きや parser 実装の違いによって
manifest の意味が変わり得る。version 1 envelope は `schema_version`、`nodes`、`reviews`、
`evidence`、`edges` を wire contract として扱うため、duplicate key を空配列や最後の値へ
暗黙に変換せず、入力エラーとして保持する必要がある。

## Decision

- version 1 manifest の5 top-level fieldについて、duplicate JSON key は
  `ValidationInputError::Json` として fail-closed にする。
- duplicate key の検査は manifest semantic validation や graph 登録より前の JSON envelope
  decode boundary とし、report、canonical graph、`--emit-manifest` artifact を生成しない。
- canonical parser は serde JSON の duplicate-field rejection を利用し、production 側に
  fieldごとの上書き・merge policyを追加しない。

## Evidence

- `parse_manifest_rejects_duplicate_top_level_fields` が `schema_version`、`nodes`、`reviews`、
  `evidence`、`edges` の5ケースを同じ parser 入口へ入力し、全て `ValidationInputError::Json`
  となることを固定した。
- focused `cargo test -p lsharp-types --test validation_input` 28件が pass。既存の explicit empty
  graph、required-field、unknown-field、nested duplicate coverage の契約と共存することを確認した。
- 公開 `lsharp validate` の `manifest_input_cli` fixture でも同じ `schema_version` duplicate を入力し、
  exit code `1`、空 stdout、`--emit-manifest` file 未生成、stderr の `duplicate` / `schema_version`
  診断を固定した。canonical parser の fail-closed contract が Rust-host CLI surface まで伝播することを
  確認した。
- production code の変更は不要だった。これは serde decode の既存 fail-closed behavior を回帰テストと
  ADRへ昇格した verified partial sliceである。

## Boundary

これは Rust version 1 manifest envelope の duplicate-key input boundary と、Rust-host 公開
`lsharp validate` の入力エラー surface に限定される。source adapter、selfhost/native manifest parser、
MCP report/exit parity、current-source stage0、Mac Apple Silicon / Linux x86_64 artifact/runtime、
EC-M2/EC-M3 aggregate の完了を意味しない。
