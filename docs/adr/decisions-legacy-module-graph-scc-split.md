# ADR: `module_graph.rs` の SCC production 分離

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/module_graph.rs`, `crates/lsharp-ir/src/module_graph/scc.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`, `decisions-legacy-module-graph-production-split.md`

## Context

path resolution の production 責務は `module_graph/resolve.rs` へ分離済みだが、親の
`module_graph.rs` には Tarjan SCC の状態、走査、deterministic group 化が残っていた。
graph mutation / dependency closure と SCC のアルゴリズムを同じ production file で
レビューすると、循環依存と compile-order の failure boundary を独立して再実行しにくい。

## Decision

- `SccState` と Tarjan の走査を `module_graph/scc.rs` の `compute_groups` へ移す。
- `ModuleGraph::scc_groups()` は既存の公開 API として残し、private helper へ委譲する。
- module 名のソート、未解決 import の無視、SCC 内のソート、dependency-first の出力順を維持する。
- helper は `pub(super)` に留め、外部公開 API と `ModuleGraph` のデータ構造を増やさない。
- 新しい seam test で wrapper を介さず deterministic SCC contract を固定し、既存 module graph tests と合わせて回帰を検証する。

## Evidence

- `module_graph::scc_tests::scc_helper_returns_stable_dependency_first_groups`: 1 passed。
- `cargo test -p lsharp-ir module_graph:: --lib`: 44 passed。
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 281 passed / 1 existing failure in the unrelated `vector-push-pair-rooted-v3` fixture.
- `cargo clippy -p lsharp-ir --all-targets -- -D warnings`、workspace check、変更対象 Rust 2024 rustfmt、`git diff --check` が pass。

## Consequences

SCC の実装と graph mutation / path resolution を独立してレビュー・focused gate 実行できる。
この slice は module graph の production 分割の一部であり、module graph 全体の parity、
incremental cache、native/selfhost gate、`I-01` / `I-08` aggregate の完了は意味しない。
