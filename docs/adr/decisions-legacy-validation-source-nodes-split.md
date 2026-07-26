# ADR: validation source graph-owned node seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-types/src/validation_source.rs`,
  `crates/lsharp-types/src/validation_source/source_nodes.rs`,
  `crates/lsharp-types/tests/validation_source.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), `LEGACY-MAINT-01`, `EC-M2-01`

## Context

`validation_source.rs` は node registration、evidence projection、typed edge projection と graph orchestration を同じ parent に保持していた。evidence と edge は既に child seam へ分離済みであり、残る graph-owned node の declaration traversal と kind mapping も分離することで、M2-01 の node identity/input boundary を変更せずに ownership と source diagnostics のレビュー範囲を明確にする必要がある。

## Decision

`Decl` の nested traversal、`:intent` / `:claim` / `:assumption` / `:open-question` の `NodeKind` mapping、typed node construction、duplicate ID と kind mismatch の検査を `validation_source/source_nodes.rs` へ移動する。parent は `mod source_nodes` 経由で node traversal を呼び、`SourceGraphError`、graph orchestration、evidence/edge child dispatch、registry helper を保持する。`source_program_to_intent_graph` の public API、node declaration order、stable ID、source span、error variants は変更しない。

## Evidence

- Baseline source adapter tests passed before the split after adding `source_adapter_preserves_every_graph_owned_node_kind`.
- RED: `mod source_nodes` を追加した状態で `cargo test -p lsharp-types --test validation_source source_adapter_preserves_every_graph_owned_node_kind -- --nocapture` を実行し、`E0583`（`source_nodes.rs` 不在）を確認。
- GREEN: node seam の移動後、全 graph-owned node kind contract と既存 source adapter tests は 24 件 pass。
- `cargo test -p lsharp-types --tests`: 221 unit + 121 integration = 342 pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、専用 workspace check、対象 Rust 2024 rustfmt、`git diff --check`、docs audit は pass。

## Consequences

graph-owned node projection の ownership が child に集約され、parent は 217 行から 156 行へ縮小した。Rust source adapter の observable behavior、node order、stable ID/kind validation、source span は維持される。selfhost/native source producer parity、manifest/runtime target gate、EC-M2-01 aggregate、I-01 / I-08 は未完了であり、`EC-M2-01` と `LEGACY-MAINT-01` は verified partial のまま継続する。
