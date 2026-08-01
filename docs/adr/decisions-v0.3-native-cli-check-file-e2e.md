# ADR: Actual selfhost CLI check file boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `selfhost/src/App/Cli.ls`,
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_actual_main_args.rs`,
  `crates/lsharp-wasm/tests/e2e/selfhost_cli_core.rs`,
  `crates/lsharp-wasm/tests/e2e/support.rs`
- Related: `V2-16c`, `LEGACY-TOOL-01`

## Context

The public `check <file>` test was ignored even though the source bundle was intended to be
the actual `App.Cli` entrypoint. The bundle also lagged behind a new `ManifestInput` import.
Separately, two recently added `Cli.ls` branches left the enclosing `defn` open, so the bundle
parser stopped at the following definition.

## Decision

Keep the existing CLI behavior and repair only the source syntax and bundle inventory. Include
`ManifestInput.ls` in the support path resolver, embedded source map, and CLI runtime module list.
Make the single-file `check` regression a normal test once its output contract is green. Keep the
native stage0, external tool, release, and target matrix boundaries separate from this Rust-host
oracle lane.

## Evidence

- `test_e2e_selfhost_cli_main_with_args_check_file` executes `check input.ls` and returns
  `Int` / `diagnostics:0` with exit success.
- `test_support_selfhost_cli_runtime_bundle_cached` verifies the cached bundle identity and
  `Tools.Validation.ManifestInput` module marker.
- `test_e2e_selfhost_cli_main_no_args_shows_help` executes the same actual bundle without argv and
  returns the `Usage: lsharp <command>` and `Commands:` help markers with exit success.
- `test_e2e_selfhost_cli_main_batched_version_and_parse_argv` compiles the bundle once, then
  verifies `--version` and `-v` return `lsharp 0.1.0`, and `parse input.ls` returns the expected
  `decls:1` / `diagnostics:0` summary.

## Boundary

This is a Rust-host actual Wasm source-bundle slice. It does not prove native stage0 `check`,
no-arg/version/parse parity, the remaining public commands, external helper parity, release
provenance, or both supported target artifacts. `V2-16c` and `LEGACY-TOOL-01` remain `[~]` in
`TODO.md`.
