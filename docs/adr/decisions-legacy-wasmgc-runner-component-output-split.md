# ADR: WasmGC runner component-output split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-wasm/src/wasmgc_runner.rs`,
  `crates/lsharp-wasm/src/wasmgc_runner_component_output.rs`,
  `crates/lsharp-wasm/src/wasmgc_runner_tests.rs`
- Related backlog: `ISSUES-HANDOFF-WASMGC` / `I-01` / `I-08`

## Context

`wasmgc_runner.rs` mixed the core `env.print-string` runner with the canonical
`wasmgc-output` module bridge, WIT Component output, Preview2 stdout/preopen
setup, CLI run-result decoding, and fd/write adapters. The 733-line file had
several independent output ownership boundaries even though the public runner
API was already stable.

## Decision

- Move canonical component-output, WIT Component, Preview2/CLI, preopen, and
  output-stream adapter logic to
  `wasmgc_runner_component_output.rs`.
- Keep the core `env.print-string` runner, core capture, and engine setup in
  `wasmgc_runner.rs`.
- Re-export every moved public function and type from the parent module so
  `lsharp_wasm::wasmgc_runner::*` paths remain unchanged.
- Add a module seam test for the CLI result decoder and preserve all existing
  WasmGC probe/runtime contracts.

## Evidence

- RED: module declaration plus seam test failed with `E0583` while the output
  module was absent.
- `cargo test -q -p lsharp-wasm --test wasmgc_probe`: 101 passed.
- `cargo test -p lsharp-wasm wasmgc_runner::tests::component_output_module_decodes_cli_exit_results`: passed.
- `cargo clippy -q -p lsharp-wasm --lib -- -D warnings` passed.
- `cargo check --workspace --quiet`, targeted Rust 2024 rustfmt, and
  `git diff --check` passed.
- `cargo test -q -p lsharp-wasm --lib`: 109 passed / 1 existing
  `RootLifetime::RootSetWithoutActiveSlot` failure.

## Consequences

`wasmgc_runner.rs` is reduced from 733 to 121 lines; the extracted output
module is 642 lines and the seam test is 27 lines. Public runner paths and
runtime semantics remain stable. Full WasmGC language/native/selfhost parity
and the broader `I-01` / `I-08` decomposition remain incomplete.
