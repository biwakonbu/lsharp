# ADR: WASI HTTP handler core emitter split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/src/wasi/http_handler.rs`,
  `crates/lsharp-wasm/src/wasi/http_handler_core.rs`,
  `crates/lsharp-wasm/src/wasi_tests/preview2.rs`
- Related: I-01, I-08, ISSUES-HANDOFF-WASMGC, `imp-06-large-file-decomposition.md`

## Context

The WASI HTTP handler module combined the public Componentization wrapper with the complete core
Wasm emitter. The 585-line parent made the HTTP boundary and the low-level section/ABI emission
harder to review independently.

## Decision

Move `emit_wasm_http_handler_core` to `wasi/http_handler_core.rs`, expose it only as a
`pub(super)` seam, and keep `http_handler::emit_wasm_http_handler_p2` at its existing module path.
The wrapper continues to componentize the same core bytes against the same WIT world.

## Evidence

- RED: the new module declaration failed with `E0583` before the child file existed.
- GREEN: HTTP handler Component compatibility test, Preview2 tests (5), and host bridge tests (7)
  passed after the move.
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`, workspace check, Rust 2024 rustfmt,
  `git diff --check`, and the documentation audit passed.

## Consequences

The parent is reduced from 585 to 23 lines and the core emitter is 566 lines. Existing public and
crate-private routing remains stable. HTTP/native/selfhost parity and the aggregate I-01/I-08
decomposition remain incomplete and stay tracked in `TODO.md`.
