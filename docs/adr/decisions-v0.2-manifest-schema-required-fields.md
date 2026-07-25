# ADR: v0.2 intent graph schema required string fields

- Status: Accepted (verified)
- Date: 2026-07-25
- Scope: `docs/schemas/intent-graph.schema.json`
- Related: `EC-M2-03`, `decisions-v0.2-validation-input-parser.md`

## Context

Rust の `parse_intent_graph_json` と graph registration は、runner、target、source commit、
artifact digest、generator、producer、tool version、timestamp の空文字を required-field
error として拒否する。一方、公開 JSON Schema は型だけを `string` として表現していたため、
schema consumer が Rust parser で拒否される manifest を先に受け入れられた。

## Decision

上記 8 フィールドへ `minLength: 1` を追加する。schema は wire-level の空文字境界を表現し、
graph-owned の参照 closure、duplicate、span ordering は引き続き Rust parser の責務とする。

## Evidence

- RED: `validation_schema::intent_graph_schema_requires_non_empty_execution_and_provenance_strings`
  が runner の `minLength` 欠落で失敗。
- GREEN: 同テスト 1件、`cargo test -p lsharp-types --test validation_input` 10件が pass。
- `git diff --check` と docs audit を通す。

## Boundary

これは schema と Rust input parser の required string parity の verified sliceである。
selfhost/native stage0 の schema consumer、Mac/Linux artifact/runtime、full M2 aggregate の完了を
意味しない。
