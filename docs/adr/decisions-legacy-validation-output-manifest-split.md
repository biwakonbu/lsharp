# ADR: validation output manifest wire seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-types/src/validation_output.rs`,
  `crates/lsharp-types/src/validation_output/manifest.rs`,
  `crates/lsharp-types/tests/validation_output.rs`
- Related: [imp-06 large-file decomposition](../development/planning/improvement-designs/imp-06-large-file-decomposition.md), `LEGACY-MAINT-01`, `EC-M2-02`, `EC-M2-03`

## Context

The public `validation_output` module owned both the `IntentGraph` serializer
API and the complete version 1 manifest wire projection. The wire types cover
nodes, evidence, execution/sampling/provenance, and every typed edge relation.
Keeping that schema machinery beside the public extension methods made the M2
manifest boundary harder to review and left the module responsible for two
different concerns.

## Decision

- Move `ManifestWire`, node/evidence/execution/provenance wire types, edge
  projection, and enum-to-string mapping to
  `crates/lsharp-types/src/validation_output/manifest.rs`.
- Declare the child explicitly with
  `#[path = "validation_output/manifest.rs"] mod manifest` and expose only
  `pub(super)` serializer helpers to the parent.
- Keep `to_manifest_json_string`, `to_manifest_json_value`, and the
  `IntentGraph` extension methods in the parent. Preserve schema version 1,
  insertion order, sampling/provenance fields, all edge relations, and the
  absence of policy fields such as `verified`.

## Evidence

- Baseline manifest output tests passed before the split.
- RED: the child module declaration before the file existed failed with
  `E0583` (`validation_output/manifest.rs` missing).
- GREEN: `empty_manifest_output_keeps_schema_boundary_without_policy_shortcut`
  plus the existing manifest contract tests: 5 passed.
- `cargo test -p lsharp-types --tests`: 340 passed.
- `cargo clippy -p lsharp-types --all-targets -- -D warnings` passed.
- Dedicated-target workspace check, targeted Rust 2024 rustfmt,
  `git diff --check`, and docs audit passed.

## Consequences

The parent is reduced from 323 to 31 lines; the private wire child is 305
lines. Public API and manifest bytes remain stable. This is a verified
maintenance slice, not completion of selfhost/native parity, release
provenance, or the aggregate `EC-M2-02` / `EC-M2-03` and `I-01` / `I-08`
boundaries.
