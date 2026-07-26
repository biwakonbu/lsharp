# ADR: WASI GC collection emission split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`,
  `crates/lsharp-wasm/src/wasi/gc_collect.rs`,
  `crates/lsharp-wasm/src/wasi/gc_collect_tests.rs`
- Related: `I-01`, `I-03`, `I-04`, `I-08`,
  `imp-06-large-file-decomposition.md`,
  `decisions-legacy-wasi-allocator-split.md`

## Context

The Wasm GC collection emitter was a 629-line production function in
`wasi.rs`. It owns root-stack scanning, mark fixed-point traversal, object-table
compaction, free-list reclamation, and collection telemetry. Those mechanics
are independent from Preview1/Preview2 orchestration and allocator wiring, but
the large parent function obscured that ownership boundary.

## Decision

- Move `emit_gc_collect_func` to `wasi/gc_collect.rs` and expose it only as
  `pub(super)`.
- Keep `CollectorGlobals`, `GcMarkHelperLocals`, layout constants, and public
  WASI entrypoints in `wasi.rs`; route compiler-world and HTTP handler callers
  through the GC module explicitly.
- Preserve root-stack traversal, mark candidate normalization, fixed-point
  scanning, object-table compaction, size-class free-list reclamation,
  collection/freed counters, function indices, and runtime exports. This is an
  ownership split only.
- Add a module seam test that emits one GC collection function body.

## Evidence

- RED: adding the GC module declaration and seam test failed with `E0583`
  while `wasi/gc_collect.rs` was absent.
- `wasi::gc_collect_tests::gc_collect_module_emits_function_body` passed.
- `runtime_collector_reuses_unrooted_allocations_across_repeated_start_series`
  passed.
- HTTP host bridge and Preview2 component smoke tests passed after explicit
  caller rewiring.
- `cargo test -p lsharp-wasm --lib` ran 107 tests with 106 passed and the
  existing `RootLifetime::RootSetWithoutActiveSlot` failure.
- Three additional collector E2E fixtures stopped before code generation with
  `RootLifetime::ImbalancedExit`; a clean `origin/main` baseline reproduced the
  representative direct-rooted-string failure, so this is outside the move.
- `cargo clippy -p lsharp-wasm --lib --quiet -- -D warnings`, workspace check,
  targeted Rust 2024 rustfmt, and `git diff --check` passed.

## Consequences

`wasi.rs` is reduced from 1687 to 1062 lines. The GC collection emitter is now
629 lines plus a 27-line seam test. Runtime semantics and public APIs remain
stable; full collector fixture parity, native/selfhost parity, and the
aggregate I-01/I-08 decomposition remain incomplete.
