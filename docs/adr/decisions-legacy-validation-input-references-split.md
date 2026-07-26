# ADR: validation input の referential-closure seam 分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-types/src/validation_input.rs`,
  `crates/lsharp-types/src/validation_input/references.rs`,
  `crates/lsharp-types/tests/validation_input.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), `LEGACY-MAINT-01`, `EC-M2-03`

## Context

`validation_input.rs` は versioned JSON manifest の wire conversion と、graph-owned
endpoint の referential closure を同じ parent に保持していた。manifest wire schema は
既に `validation_input/manifest.rs` へ分離済みであり、残る evidence subject / edge endpoint
の検証と missing-node 診断も分離して、M2-03 の入力境界を wire conversion と graph closure
の責務軸でレビューできる状態にする必要がある。

## Decision

evidence subject の node closure、edge relation ごとの graph-owned endpoint 検証、
`require_node` / `has_node` / `missing_node` を `validation_input/references.rs` へ移動する。
parent は manifest parse orchestration、node/evidence/edge canonical construction、
`ValidationInputError` の公開 API を保持し、`mod references` 経由の `pub(super)` dispatch
だけを追加する。既存の `parse_intent_graph_json` API、edge 順序、stable ID、relation 名、
`MissingNodeReference` の診断 semantics は変更しない。

## Evidence

- Baseline の manifest input tests は追加 contract test 前後で成功した。
- RED: `mod references` を追加した状態で
  `cargo test -p lsharp-types --test validation_input parse_manifest_reports_missing_edge_endpoint_relation_and_id`
  を実行し、`E0583`（`references.rs` 不在）を確認。
- GREEN: referential-closure seam の移動後、欠落 `constrained-by.assumption` の relation と
  stable ID を固定する contract を含む `validation_input` 16 tests が pass。
- `cargo test -p lsharp-types --tests`: 221 unit + 123 integration = 344 pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、専用 workspace check、対象
  Rust 2024 rustfmt、`git diff --check` は pass。

## Consequences

参照閉包と欠落 node 診断の ownership が child に集約され、parent は 321 行から 253 行へ
縮小した。Rust manifest parser の observable behavior、edge relation、stable ID、diagnostic
boundary は維持される。selfhost/native producer parity、manifest/runtime target gate、
EC-M2-03 aggregate、I-01 / I-08 は未完了であり、`EC-M2-03` と `LEGACY-MAINT-01` は
verified partial のまま継続する。
