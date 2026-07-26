# ADR: WASI GC collector core split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/src/wasi/gc_collect.rs`,
  `crates/lsharp-wasm/src/wasi/gc_collect_core.rs`,
  `crates/lsharp-wasm/src/wasi/gc_collect_tests.rs`
- Related: I-01, I-08, ISSUES-HANDOFF-WASMGC, `imp-06-large-file-decomposition.md`

## Context

`gc_collect.rs` contained the complete low-level mark/sweep emitter in one 629-line parent. The
collector is called by the compiler-world emitter and has a stable `CollectorGlobals` index ABI,
so the implementation can be isolated without changing the caller contract.

## Decision

Move the collector body to `wasi/gc_collect_core.rs`, expose it only through a `pub(super)`
function, and retain `gc_collect::emit_gc_collect_func` as the stable module-level entrypoint.
Preserve root seeding, fixed-point marking, free-list growth, and metric updates byte-for-byte.

## Evidence

- RED: the module declaration failed with `E0583` before the child file existed.
- GREEN: collector focused tests (2), `cargo check -p lsharp-wasm --lib`, and the existing full
  runtime coverage were executed after the move.
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`, workspace check, Rust 2024 rustfmt,
  `git diff --check`, and the documentation audit passed.

## Consequences

The parent is reduced from 629 to 8 lines and the collector child is 629 lines. Existing caller
paths and global-index semantics remain stable. GC/native/selfhost parity and the aggregate
I-01/I-08 decomposition remain incomplete and stay tracked in `TODO.md`.
