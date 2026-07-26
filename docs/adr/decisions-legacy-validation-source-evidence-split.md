# ADR: validation source evidence seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_source.rs`, `crates/lsharp-types/src/validation_source/source_evidence.rs`, `crates/lsharp-types/tests/validation_source.rs`

## Context

`validation_source.rs` は source node registry、evidence record の canonical projection、typed edge の三つの責務を同じ parent に保持していた。M2-02 の evidence input contract を変えずに、evidence construction の ownership を分離し、後続の selfhost/native producer parity と manifest boundary の差分を追いやすくする必要がある。

## Decision

evidence record の declaration traversal、`Evidence` construction、subject/method/outcome/independence の enum parsing を `validation_source/source_evidence.rs` に移動する。parent は `mod source_evidence` 経由で traversal を呼び、`SourceGraphError`、`require_node`、`IntentGraph` の orchestration と typed edge projection を保持する。public `source_program_to_intent_graph`、error variants、source span、required field/registry closure は変更しない。

## Evidence

- RED: `mod source_evidence` を追加した状態で `cargo test -p lsharp-types --test validation_source source_adapter_preserves_evidence_sampling_projection_through_public_seam` を実行し、`E0583`（`source_evidence.rs` 不在）を確認。
- GREEN: evidence seam の移動後、sampling/provenance contract test は pass。移動元 origin/main の evidence body と child body の比較も一致した。
- `cargo test -p lsharp-types --tests`: 339 pass。
- `cargo clippy -p lsharp-types --all-targets -- -D warnings`、workspace `cargo check`、対象 Rust 2024 rustfmt、`git diff --check`、docs audit は pass。

## Consequences

evidence projection の ownership が child に集約され、`validation_source.rs` は 504 行から 336 行へ縮小した。Rust source adapter の observable behavior、sampling/provenance values、required evidence/registry diagnostics は維持される。selfhost/native producer parity、manifest/runtime target gate、EC-M2 aggregate は未完了であり、`EC-M2-02` と `LEGACY-MAINT-01` は verified partial のまま継続する。
