# ADR: WASI allocator emission split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`,
  `crates/lsharp-wasm/src/wasi/allocator.rs`,
  `crates/lsharp-wasm/src/wasi/allocator_tests.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-wasi-http-handler-split.md`

## Context

The WASI backend's `__alloc` emitter had become a 385-line production block in
`wasi.rs`. It owns the size-class free-list and oversize first-fit path, while
the caller only needs to provide the established `AllocatorGlobals` indices.
Keeping that implementation in the parent made the remaining WASI production
split harder to review and coupled allocator ownership to Preview1/Preview2
orchestration.

## Decision

- Move `emit_alloc_func` to `wasi/allocator.rs` and expose it only as
  `pub(super)`.
- Keep `AllocatorGlobals`, memory-layout constants, and public WASI entrypoints
  in `wasi.rs`; route both compiler-world and HTTP handler code generation
  through the allocator module explicitly.
- Preserve function ordering, allocator/global indices, size-class selection,
  oversize first-fit behavior, telemetry updates, memory-growth behavior, and
  tagged pointer ABI. This is an ownership split only.
- Add a module seam test that verifies the allocator emits one Wasm function
  body before relying on the existing runtime regression tests.

## Evidence

- RED: adding the allocator module declaration and seam test failed with
  `E0583` while `wasi/allocator.rs` was absent.
- `wasi::allocator_tests::allocator_module_emits_function_body` passed.
- `runtime_allocator_reuses_small_blocks_without_linear_scan` passed.
- `runtime_allocator_uses_oversize_fallback_scan` passed.
- `test_e2e_alloc_basic` passed.
- Existing HTTP host bridge and Preview2 component smoke tests passed after the
  explicit allocator call-site rewire.
- `cargo test -p lsharp-wasm --lib` ran 106 tests with 105 passed and the
  pre-existing `RootLifetime::RootSetWithoutActiveSlot` failure.
- `cargo clippy -p lsharp-wasm --lib --quiet -- -D warnings`, targeted Rust
  2024 rustfmt, and `git diff --check` passed.

## Consequences

`wasi.rs` is reduced from 2071 to 1687 lines. The allocator implementation is
now 388 lines plus a 24-line seam test. The public API and runtime contract are
unchanged; GC collection emission, native/selfhost parity, and the aggregate
I-01/I-08 decomposition remain incomplete.
