# ADR: Actual selfhost CLI artifact output boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs`,
  `selfhost/src/App/Cli.ls`
- Related: `V2-16b`, `LEGACY-IO-01`, `LEGACY-COMP-01`

## Context

The actual `App.Cli` source bundle had routing coverage for `compile` and `build`, but the
artifact boundary must distinguish a real Preview1 Wasm file from a size/summary response. The
component target also depends on external packaging and must not be reported as successful while
that boundary is unavailable.

## Decision

Keep `compile` and `build` on the actual CLI path and require their `-o` output to be valid,
runnable Preview1 Wasm. Keep the component target explicit and fail closed until external
component packaging is connected. The test compiles the source bundle once and reuses it for both
commands and the component rejection case.

## Evidence

- `test_e2e_selfhost_cli_main_compile_and_build_output_actual_preview1_wasm` verifies `compile`
  and `build` output magic, `wasmparser` validation, standalone WASI execution, exit `0`, and
  empty stdout.
- The same test verifies `compile --target wasi-component -o ...` returns exit `1` and writes no
  component artifact while packaging is external.

## Boundary

This is a Rust-host actual Wasm source-bundle artifact/runtime slice. It does not prove native
stage0 output-path behavior, full-program native entrypoint parity, component sidecar generation,
release provenance, or both supported target artifacts. `V2-16b`, `LEGACY-IO-01`, and
`LEGACY-COMP-01` remain `[~]` in `TODO.md`.
