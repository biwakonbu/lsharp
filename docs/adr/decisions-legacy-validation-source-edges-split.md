# ADR: validation source typed edge seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_source.rs`,
  `crates/lsharp-types/src/validation_source/source_edges.rs`,
  `crates/lsharp-types/tests/validation_source.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), `LEGACY-MAINT-01`, `EC-M2-02`

## Context

`validation_source.rs` は source node registry、evidence projection、typed edge projection の三つの責務を同じ parent に保持していた。evidence projection は既に `source_evidence.rs` へ分離済みであり、残る edge traversal と endpoint validation も別 seam にすることで、M2-02 の source graph 入力境界を変更せずに ownership と diagnostics のレビュー範囲を明確にする必要がある。

## Decision

typed edge の declaration traversal、`:motivates` / `:constrained-by` / `:tested-by` / `:supports` / `:contradicts` の projection、stable ID parsing、node/evidence registry closure を `validation_source/source_edges.rs` へ移動する。parent は `mod source_edges` 経由で edge traversal を呼び、`SourceGraphError`、node registration、evidence orchestration、`require_node` / `require_evidence` helper を保持する。`source_program_to_intent_graph` の public API、edge order、error variants、source span、registry closure は変更しない。

## Evidence

- Baseline source adapter tests passed before the split after adding the `:contradicts` success fixture.
- RED: `mod source_edges` を追加した状態で `cargo test -p lsharp-types --test validation_source source_adapter_registers_contradicts_evidence_edge -- --nocapture` を実行し、`E0583`（`source_edges.rs` 不在）を確認。
- GREEN: edge seam の移動後、`:contradicts` contract test と既存 source adapter tests は 23 件 pass。
- `cargo test -p lsharp-types --tests`: 221 unit + 120 integration = 341 pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、専用 workspace check、対象 Rust 2024 rustfmt、`git diff --check`、docs audit は pass。

## Consequences

typed edge projection の ownership が child に集約され、parent は 336 行から 217 行へ縮小した。Rust source adapter の observable behavior、edge insertion order、typed endpoint diagnostics、source span は維持される。selfhost/native producer parity、manifest/runtime target gate、EC-M2-02 aggregate、I-01 / I-08 は未完了であり、`EC-M2-02` と `LEGACY-MAINT-01` は verified partial のまま継続する。
