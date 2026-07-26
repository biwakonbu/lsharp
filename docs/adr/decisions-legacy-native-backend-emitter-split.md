# ADR: native backend emitter split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-tooling/src/native.rs`,
  `crates/lsharp-tooling/src/native_emitter.rs`,
  `crates/lsharp-tooling/src/native_tests.rs`
- Related backlog: `ISSUES-HANDOFF-LOW-RISK` / `I-01` / `I-08`

## Context

After the native backend test extraction, `native.rs` still contained the
Apple Silicon `NativeFunctionEmitter` state machine and all instruction
emission branches. That target-specific emitter occupied more than 550 lines
and obscured the parent module's orchestration, reachability, symbol, and
temporary-artifact responsibilities.

## Decision

- Move `NativeIfFrame` and `NativeFunctionEmitter` to
  `crates/lsharp-tooling/src/native_emitter.rs` behind the same
  `macos`/`aarch64` configuration.
- Keep native module assembly, reachable-function discovery, symbol helpers,
  stack/heap assembly primitives, temporary paths, and the public compile
  boundary in `native.rs`.
- Use a private path module and `pub(super)` only for the emitter constructor
  and entrypoint. The parent calls the emitter explicitly through the module;
  no public API, diagnostic text, instruction mapping, or assembly contract is
  changed.
- Add a seam test that invokes the extracted emitter for an exported `main`
  function and checks the existing label, constant, and return-label output.

## Evidence

- RED: adding the module declaration and seam test before the file existed
  failed with `E0583` (`native_emitter` module not found).
- `cargo test -q -p lsharp-tooling native_ --lib -- --nocapture`: 12 passed.
- `cargo test -q -p lsharp-tooling test_compile_file_native_target --lib -- --nocapture`: 8 passed.
- `cargo test -q -p lsharp-tooling`: 134 passed.
- `cargo clippy -q -p lsharp-tooling --lib --tests -- -D warnings` passed.
- `cargo check --workspace --quiet`, targeted Rust 2024 rustfmt, and
  `git diff --check` passed.

## Consequences

`native.rs` is reduced from 855 to 304 lines; the extracted emitter is 561
lines. The native test namespace and Apple Silicon target gate remain stable.
This is an ownership-only refactor: native/selfhost parity, Linux native
support, and the broader `I-01` / `I-08` decomposition remain incomplete.
