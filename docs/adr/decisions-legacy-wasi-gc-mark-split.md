# ADR: WASI GC mark-candidate helper split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`, `crates/lsharp-wasm/src/wasi/gc_mark.rs`
- Related: `I-03`, `I-04`, `decisions-legacy-wasi-free-list-split.md`

## Context

The collector's mark phase converts raw and tagged candidate values to heap addresses,
searches the object table, and transitions matching entries to `pending`. That code-emission
helper is called from root, reference, vector, record/ADT, closure, and hashmap traversal in
`emit_gc_collect_func`, but was embedded in the same production file as the full collector.

## Decision

- Move `emit_gc_mark_candidate` to `wasi/gc_mark.rs`.
- Keep `CollectorGlobals` and `GcMarkHelperLocals` as the parent-owned index contracts, with the
  module helper remaining `pub(super)`.
- Preserve tagged-pointer normalization, heap range checks, object-table stride/mark offset,
  and all caller local/global indices without changing collector function ordering or ABI.
- Add a seam test that emits one mark-candidate body through the new module.

## Evidence

- RED: the new seam test failed while `gc_mark.rs` was absent (`E0583`); a missed caller was
  then corrected before GREEN.
- GREEN: `wasi::gc_mark_tests::gc_mark_module_emits_mark_candidate_body` (1 passed).
- `cargo test -p lsharp-wasm --test e2e selfhost_gc_collect_ -- --nocapture`: 3 passed.
- `cargo test -p lsharp-wasm wasi:: --lib`: 43 passed / 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure.

## Consequences

The WASI production parent is reduced from 2911 lines to 2792 lines and mark-candidate
emission can be reviewed independently from collector orchestration. This slice does not
complete full GC parity, allocator growth, native ABI, dynamic memory layout, Mac/Linux stage0,
or the aggregate I-03/I-04 requirements. The known root-lifetime failure remains outside this
refactor.
