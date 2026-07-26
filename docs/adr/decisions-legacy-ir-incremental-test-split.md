# ADR: lsharp-ir incremental regression test split

- Status: Accepted (verified partial slice)
- Date: 2026-07-26
- Scope: `crates/lsharp-ir/src/lib_tests.rs`,
  `crates/lsharp-ir/src/lib_tests/incremental_compile.rs`,
  `crates/lsharp-ir/src/lib_tests/incremental_analysis.rs`
- Related: `I-01`, `I-08`, `imp-06-large-file-decomposition.md`,
  `decisions-legacy-ir-lib-test-split.md`,
  `decisions-legacy-ir-linker-test-split.md`,
  `decisions-legacy-ir-multifile-test-split.md`

## Context

After the linker and multi-file suites were extracted, `lib_tests.rs` still
contained the 25-test incremental suite. Cache-hit compile tests, source
override analysis tests, SCC type-surface reuse, segmented lowering, and link
cache regressions had different ownership boundaries but were kept in one
module.

## Decision

- Move the ten cache/basic compile regressions to
  `lib_tests/incremental_compile.rs` and keep the
  `incremental_compile_tests` module name through a `#[path]` declaration.
- Move the remaining fifteen analysis, segmented reuse, link-cache, and
  formatter-boundary regressions to `lib_tests/incremental_analysis.rs`.
  These tests use the explicit `incremental_analysis_tests` module name so the
  new ownership boundary is visible in focused test output; their old module
  path is intentionally changed.
- Keep compiler, linker, cache, and fixture behavior unchanged. The split is a
  test ownership change only; the small `main_function` and `call_positions`
  helpers remain with the compile suite where they are used.

## Evidence

- RED: the new `incremental_analysis_tests` path failed with `E0583` while the
  file was absent.
- GREEN: `cargo test -p lsharp-ir incremental_compile_tests --lib` passed all
  10 compile tests.
- The analysis focused lane ran 15 tests; 14 passed and the formatter smoke
  test reproduced the pre-existing `IntentSource.ls` undefined
  `vector-push-pair-rooted-v3` diagnostic.
- `RUST_MIN_STACK=33554432 cargo test -p lsharp-ir --lib` ran 283 tests with
  282 passed and the same one pre-existing fixture failure.
- `cargo clippy -p lsharp-ir --all-targets --quiet -- -D warnings`,
  `cargo check --workspace --quiet`, targeted Rust 2024 rustfmt, and
  `git diff --check` passed.

## Consequences

`lib_tests.rs` is reduced from 1396 to 314 lines. The extracted compile and
analysis modules are 416 and 650 lines, respectively, both below the 800-line
maintenance limit. The remaining test modules, IR production decomposition,
and the I-01/I-08 aggregate remain incomplete.
