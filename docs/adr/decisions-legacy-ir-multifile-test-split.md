# ADR: lsharp-ir multi-file compile test module split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib_tests.rs`,
  `crates/lsharp-ir/src/lib_tests/multifile_compile.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-ir-lib-test-split.md`,
  `decisions-legacy-ir-linker-test-split.md`

## Context

After the linker suite was separated, `lib_tests.rs` still combined multi-file
module graph, SCC visibility, modular lowering, and incremental compilation
regressions with unrelated fingerprint, cache, memory, and selfhost tests. The
multi-file suite is a distinct ownership boundary and is also the largest
remaining test module in this file.

## Decision

- Move the `multifile_compile_tests` module to
  `lib_tests/multifile_compile.rs`.
- Preserve the module name and test names through a `#[path]` declaration so
  test filters and private helper access remain unchanged.
- Keep compiler/linker behavior and all fixtures unchanged; this is a test
  ownership split only.

## Evidence

- RED: the new path declaration failed with `E0583` while
  `multifile_compile_split.rs` was absent.
- GREEN: `cargo test -p lsharp-ir multifile_compile_tests --lib` passed all 13
  multi-file tests.
- Rust 2024 `rustfmt --check` and `git diff --check` passed. The large-stack
  package gate remains 282 passed with the pre-existing
  `vector-push-pair-rooted-v3` incremental fixture failure.

## Consequences

`lib_tests.rs` is reduced from 2000 to 1396 lines, while the extracted
multi-file module is 596 lines. Five unrelated test modules remain in the
parent for later low-conflict slices; IR production decomposition and the
I-01/I-08 aggregate remain incomplete.
