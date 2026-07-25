# ADR: native backend test module split

- Status: Accepted (verified maintenance slice)
- Date: 2026-07-25
- Scope: `crates/lsharp-tooling/src/native.rs`
- Related backlog: `ISSUES-HANDOFF-LOW-RISK` / `I-01` / `I-08`

## Context

`lsharp-tooling/src/native.rs` contained the native backend implementation and
three macOS Apple Silicon-only unit tests in the same file. The implementation
was 963 lines including the test module, which made the production boundary
harder to review and added test-only filesystem/linker fixtures to the backend
source.

## Decision

Move the existing `native::tests` module to
`crates/lsharp-tooling/src/native_tests.rs` using a `#[path]` module declaration.
Keep the parent module's `macos`/`aarch64` configuration and the
`native::tests::*` namespace unchanged. Do not change native code generation,
diagnostic behavior, output paths, or public APIs.

## Evidence

- `cargo test -p lsharp-tooling native_ --lib -- --nocapture`: 11 passed (the
  three native backend tests plus eight matching native compile tests).
- `cargo clippy -p lsharp-tooling --lib --tests -- -D warnings`: passed.
- Targeted Rust 2024 `rustfmt` and `git diff --check`: passed.
- `native.rs` decreased from 963 to 855 lines; the extracted test module is
  108 lines.

The first baseline attempt was interrupted by the host filesystem reaching
zero free space during a fresh dependency build; it did not reach test
execution. The post-change focused suite completed after disabling test debug
symbols/incremental artifacts for the isolated worktree. No behavior change is
present in the diff.

## Consequences

The native backend production file now has a clear test ownership boundary,
while platform gating and test module paths remain stable. Native backend
production responsibility decomposition and the broader `I-01` / `I-08`
aggregate remain open; this ADR records only the verified test extraction.
