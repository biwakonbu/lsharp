# ADR: WASI HTTP handler component builder split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasi.rs`,
  `crates/lsharp-wasm/src/wasi/http_handler.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-wasi-instructions-split.md`

## Context

The WASI backend's HTTP handler world builder is an independent component
boundary from Preview1/Preview2 compiler-world emission and the allocator/GC
runtime helpers. It still occupied roughly 583 lines in the large `wasi.rs`
production module and reached into the same private runtime helpers.

## Decision

- Move `emit_wasm_http_handler_core` and the HTTP component builder internals to
  `wasi/http_handler.rs`.
- Keep `is_http_handler_module` and the public
  `emit_wasm_http_handler_p2` wrapper in `wasi.rs`, so the existing public API,
  HTTP detection, and call sites remain unchanged.
- Preserve all HTTP import/export indices, function ordering, allocator/GC
  wiring, component metadata, and WIT world selection. This is a production
  ownership split only.

## Evidence

- RED: adding the module declaration failed with `E0583` while
  `wasi/http_handler.rs` was absent.
- `cargo test -p lsharp-wasm --lib host_bridge::tests::test_http_handler_world_calls_lsharp_handle_and_sets_response_outparam`
  passed.
- `cargo test -p lsharp-wasm --lib host_bridge::tests::test_http_handler_world_instantiates_dummy_component_against_synthetic_host`
  passed.
- `cargo test -p lsharp-wasm wasi::tests::test_emit_wasm_wasi_p2_basic_program_compiles --lib`
  passed.
- `cargo test -p lsharp-tooling test_compile_file_handle_only_emits_http_handler_component_export`
  passed.
- `cargo test -p lsharp-wasm --lib` ran 105 tests with 104 passed and the
  pre-existing `RootLifetime::RootSetWithoutActiveSlot` failure.
- `cargo clippy -p lsharp-wasm --lib --quiet -- -D warnings`, workspace check,
  targeted Rust 2024 rustfmt, and `git diff --check` passed.

## Consequences

`wasi.rs` is reduced from 2649 to 2071 lines and the extracted HTTP handler
module is 585 lines. The public API and component runtime contract remain
stable; the remaining WASI production decomposition, native/selfhost parity,
and I-01/I-08 aggregate remain incomplete.
