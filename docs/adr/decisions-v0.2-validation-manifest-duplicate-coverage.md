# ADR: v0.2 validation manifest duplicate coverage boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-29
- Scope: `crates/lsharp-types/src/validation_input/manifest.rs` の version 1 JSON manifest input
- Related: `EC-M2-03`、`docs/adr/decisions-v0.2-native-validation-evidence-duplicate-coverage-parser.md`

## Context

source parser では duplicate coverage bucket を parser-owned error として拒否している一方、
version 1 JSON manifest の `coverage` は `BTreeMap` へ直接 deserialize していた。JSON object の
同一 key は map 化の際に後続値で上書きされるため、重複した入力が canonical graph と異なる値へ
黙って変質する余地があった。

## Decision

- manifest の `sampling.coverage` は `UniqueCoverage` serde visitor を通して map 化する。
- visitor は同じ bucket key を二度観測した時点で decode error を返し、
  `parse_intent_graph_json` は `ValidationInputError::Json` として graph 登録前に fail-closed にする。
- canonical `SamplingPlan` は引き続き `BTreeMap` を保持し、duplicate を後段で再判定しない。
  source parser の duplicate policy と JSON wire parser の duplicate policy をそれぞれ入力境界の責務に
  留める。

## Evidence

- `parse_manifest_rejects_duplicate_coverage_bucket_keys` を先に RED として追加し、実装前は duplicate
  key が後続値へ上書きされて parse 成功することを確認した。
- `cargo test -p lsharp-types --test validation_input -- --nocapture`（18 passed）。
- `cargo test -p lsharp-types`（unit 221件、integration/doc testsを含め全通過）。
- 対象3ファイルの `rustfmt --edition 2024 --check` と `git diff --check` が通過した。
- `validate_rejects_duplicate_coverage_bucket_without_report_or_manifest_output` は公開
  `lsharp validate` でも exit `1`、空 stdout、manifest file なし、`coverage` と duplicate-key
  分類を含む stderr になることを固定した。
- `cargo test -p lsharp-driver --test validate_cli`（27 tests）。
- `cargo clippy -p lsharp-driver --test validate_cli -- -D warnings`。

## Boundary and follow-up

これは Rust canonical manifest input と公開 Rust CLI の duplicate coverage wire boundary に限定した verified
partial sliceである。selfhost/native manifest parity、report/atomic writer、coverage count/cases の意味論、
current-source artifact/runtime、Mac/Linux matrix、EC-M2-03 aggregate は未完了であり、TODO の `[~]` を
維持する。

## 非目標

- `sum(coverage counts) == cases` や count 上限をこの変更で決定しない。
- source parser の duplicate diagnostic code/span を変更しない。
- MCP、filesystem writer、stage0 provenance、supported target matrixを先取りしない。
