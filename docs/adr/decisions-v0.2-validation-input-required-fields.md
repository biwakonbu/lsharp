# ADR: v0.2 validation manifest の top-level required fields

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_input.rs` の version 1 manifest envelope
- Related: `EC-M2-03`, `docs/adr/decisions-v0.2-validation-input-parser.md`,
  `docs/schemas/intent-graph-manifest.schema.json`

## Context

version 1 manifest は `schema_version`、`nodes`、`evidence`、`edges` を envelope の required
fields として持つ。unknown field と unsupported version は既に検証していたが、いずれかを欠落
させた JSON が parser 入口で fail-closed になることを、同一 fixture の回帰テストで列挙して
いなかった。欠落を default や空配列へ暗黙変換すると、producer の破損と意図した empty graph を
区別できない。

## Decision

- version 1 manifest の4 top-level fields は serde decode 上 required とし、欠落時は
  `ValidationInputError::Json` を返す。
- `nodes` / `evidence` / `edges` の欠落を空配列へ補完せず、`schema_version` の欠落も既定 version
  へ丸めない。明示された空配列だけを empty graph として受理する。
- field の存在チェックは JSON envelope decode に委譲し、schema version / graph semantic validation
  とは別の failure boundary として保持する。

## Evidence

- `parse_manifest_rejects_missing_top_level_required_fields` は complete manifest を JSON object
  として fieldごとに1つ削除し、4ケース全てが `ValidationInputError::Json` になることを検証する。
- RED のテスト追加後、production code を変更せず focused test が GREEN になった。explicit empty
  graph の既存契約と、欠落 envelope の拒否を分離できることを固定した。
- 実行: `cargo test -p lsharp-types --test validation_input parse_manifest_rejects_missing_top_level_required_fields -- --nocapture`

## Boundary

これは Rust manifest envelope の required-field decode に限定した verified slice である。source
syntax adapter、selfhost/native stage0、CLI/MCP report/exit parity、schema migration、Mac Apple
Silicon / Linux x86_64 artifact/runtime evidence、EC-M2 aggregate の完了を意味しない。
