# ADR: module graph mutation and dirty-set split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/module_graph.rs`, `crates/lsharp-ir/src/module_graph/mutation.rs`
- Related: `I-01`, `I-08`, `decisions-legacy-module-graph-scc-split.md`

## Context

`ModuleGraph` combines graph construction/traversal with incremental mutation operations:
dependency and reverse-dependency closures, dirty-set expansion, reverse-index rebuild,
import diffs, import updates, and module removal. These operations form the incremental
compilation boundary and are independent from cycle/topological/SCC traversal.

## Decision

- Move the mutation, dependency-closure, dirty-set, and formatter-trio expansion methods to
  `module_graph/mutation.rs`.
- Keep the `ModuleGraph` fields private and preserve the existing public method signatures,
  stable sorting, formatter-trio atomic expansion, reverse-dependency rebuild, and diagnostics.
- Expose only the reverse-index rebuild to the sibling resolver module through the narrowest
  parent-scoped visibility needed by the existing resolver path.
- Add a module seam test for deterministic import diffs.

## Evidence

- RED: the new mutation seam failed while `mutation.rs` was absent (`E0583`).
- GREEN: `module_graph::mutation_tests::mutation_module_reports_stable_import_diff` (1 passed).
- `cargo test -p lsharp-ir module_graph:: --lib`: 45 passed.
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 282 passed; 1 pre-existing
  `vector-push-pair-rooted-v3` fixture failure remains in incremental compilation.
- `cargo clippy -p lsharp-ir --all-targets --quiet -- -D warnings`, `cargo check --workspace
  --quiet`, Rust 2024 `rustfmt --check`, `git diff --check`, and `bash scripts/audit_docs.sh`
  passed.

## Consequences

The module-graph production parent is reduced from 478 lines to 316 lines. Incremental
mutation and dirty-set behavior can be reviewed independently from traversal/SCC logic. This
slice does not complete the full incremental cache contract, native/selfhost parity, or the
aggregate I-01/I-08 requirements.
