# ADR: WasmGC runner Preview2 and CLI split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-27
- Scope: `crates/lsharp-wasm/src/wasmgc_runner_component_output.rs`,
  `crates/lsharp-wasm/src/wasmgc_runner_component_preview2.rs`,
  `crates/lsharp-wasm/src/wasmgc_runner_tests.rs`
- Related: I-01, I-08, ISSUES-HANDOFF-WASMGC, `imp-06-large-file-decomposition.md`

## Context

`wasmgc_runner_component_output.rs` mixed the canonical core-module and WIT Component output
sinks with Preview2 context construction, preopen rights, stdout stream handling, and CLI exit
decoding. This made the public output boundary difficult to review while the parent continued to
grow.

## Decision

Move Preview2/CLI Component execution, `Preview2Preopen` and rights types, the Preview2 stdout
stream adapter, and the `wasi:cli/run` result decoder to
`wasmgc_runner_component_preview2.rs`. Re-export the existing public functions and types from the
parent module, and keep the decoder available through the existing test-only path. Preserve the
existing entry-point, preopen, stdout, and exit-code contracts.

## Evidence

- RED: adding the module declaration failed with `E0583` until the child module was created.
- GREEN: Preview2 rights and CLI decoder focused tests (2), `wasmgc_probe` (101), and
  `cargo check -p lsharp-wasm --lib` passed.
- `cargo test -p lsharp-wasm --lib`: 111 passed; 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure remains in the origin/main baseline.
- `cargo clippy -p lsharp-wasm --lib -- -D warnings`, workspace check, Rust 2024 rustfmt,
  `git diff --check`, and the documentation audit passed.

## Consequences

The parent is reduced from 543 to 189 lines and the Preview2/CLI child is 375 lines. Public module
paths and runtime semantics remain stable. Full WasmGC language/native/selfhost parity, advanced
runtime handoff, and the aggregate I-01/I-08 decomposition remain incomplete and stay tracked in
`TODO.md`.
