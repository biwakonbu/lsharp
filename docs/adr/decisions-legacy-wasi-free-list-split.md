# ADR: WASI allocator free-list helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi/free_list.rs`
- Related: `I-03`, `I-04`, `decisions-legacy-wasi-structs-split.md`

## Context

The WASI allocator and GC collector share four linear-memory free-list emission helpers:
class selection, bump-capacity preservation, small-class pop, and class push. Keeping those
helpers in `wasi.rs` couples allocator implementation changes to the larger GC/runtime
orchestration and makes the free-list ABI boundary harder to review independently.

## Decision

- Move the four free-list code-emission helpers to `wasi/free_list.rs`.
- Keep the helpers `pub(super)` and reuse the parent size-class constants; callers retain the
  same local/global indices and memory offsets.
- Update both `__alloc` and GC sweep callers through the module without changing function
  ordering, free-list telemetry, size classes, or the linear-memory ABI.
- Add a seam test that emits the capacity-preservation body through the new module.

## Evidence

- RED: the new seam test failed while `free_list.rs` was absent (`E0583`).
- GREEN: `wasi::free_list_tests::free_list_module_emits_capacity_copy_body` (1 passed).
- `cargo test -p lsharp-wasm --test e2e runtime_allocator_size_classes -- --nocapture`:
  2 passed (small-class O(1) reuse and oversize fallback scan).
- `cargo test -p lsharp-wasm wasi:: --lib`: 42 passed / 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure.

## Consequences

The WASI production parent is reduced from 3015 lines to 2911 lines and free-list emission
can be reviewed and tested independently from allocator/GC orchestration. This slice does
not complete allocator growth, GC parity, native ABI, dynamic memory layout, Mac/Linux stage0,
or the aggregate I-03/I-04 requirements. The known root-lifetime failure remains outside this
refactor.
