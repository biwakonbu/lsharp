# ADR: native emitter memory/struct instruction seam の分割

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-tooling/src/native_emitter.rs`,
  `crates/lsharp-tooling/src/native_emitter/memory.rs`,
  `crates/lsharp-tooling/src/native_tests.rs`
- Related backlog: `LEGACY-MAINT-01` / `I-01` / `I-08`

## Context

The Apple Silicon `NativeFunctionEmitter` had already been separated from
`native.rs`, but its instruction dispatcher still mixed scalar/control-flow
emission with struct allocation/access and linear-memory load/store helpers.
Keeping those memory-oriented operations in the parent made the emitter seam
harder to review and kept the production file above the repository's preferred
size range.

## Decision

- Move `emit_struct_new`, `emit_struct_get`, `emit_struct_set`, the typed load
  and store helpers, and `native_struct_field_count` to
  `crates/lsharp-tooling/src/native_emitter/memory.rs`.
- Declare the child explicitly with
  `#[path = "native_emitter/memory.rs"] mod memory` under the existing Mac
  Apple Silicon configuration. Keep the child private and expose only the
  parent-facing methods as `pub(super)`.
- Preserve the existing instruction mapping, stack-depth accounting, heap
  allocation sequence, struct field bounds checks, and linear-memory ABI.
  This is an ownership-only refactor with no public API or assembly contract
  change.

## Evidence

- RED: adding the module declaration and struct/memory seam test before the
  child file existed failed with `E0583` (`native_emitter/memory.rs` missing).
- GREEN: `native_emitter_memory_seam_preserves_struct_access_contract` checks
  heap-page addressing and the existing struct allocation/access assembly.
- `cargo test -p lsharp-tooling --lib -- --nocapture`: 136 passed.
- `cargo clippy -p lsharp-tooling --all-targets -- -D warnings` passed.
- Dedicated-target `cargo check --workspace` passed.
- Targeted Rust 2024 rustfmt, `git diff --check`, and `bash scripts/audit_docs.sh`
  passed.

## Consequences

`native_emitter.rs` is reduced from 561 to 441 lines and the new memory seam is
141 lines. The Mac Apple Silicon native compile gate remains covered. Linux
native support, full language/native/selfhost parity, and the aggregate
`I-01` / `I-08` decomposition remain incomplete, so `LEGACY-MAINT-01` stays
`[~]` in `TODO.md`.
