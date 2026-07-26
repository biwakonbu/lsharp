# ADR: lsharp-ir linker test module split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib_tests.rs`, `crates/lsharp-ir/src/lib_tests/linker.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-ir-lib-test-split.md`

## Context

The previous `lib.rs` test-only extraction left all seven test modules in one
`lib_tests.rs` file. The linker regression suite is an independent ownership
boundary from import, multi-file, incremental, fingerprint, memory-instruction,
and selfhost-collision tests.

## Decision

- Move the `linker_tests` module to `lib_tests/linker.rs`.
- Keep the module name and test names stable through a `#[path]` declaration so
  existing test filters and private helper access continue to work.
- Keep production IR/linking behavior unchanged; this is a test ownership split
  only.

## Evidence

- RED: the new path declaration failed with `E0583` while `linker_split.rs` was
  absent.
- GREEN: `cargo test -p lsharp-ir linker_tests --lib` passed all 7 linker tests.
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib`: 282 passed; the
  pre-existing `vector-push-pair-rooted-v3` incremental fixture failure remains.
- Rust 2024 `rustfmt --check` and `git diff --check` passed.

## Consequences

`lib_tests.rs` is reduced from 2384 to 2000 lines, while the extracted linker
module is 383 lines. The remaining six test modules stay in `lib_tests.rs` for
subsequent low-conflict slices; the broader IR production split and I-01/I-08
aggregate remain incomplete.
