# ADR: Native standalone command-line argv boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-04
- Scope: standalone Preview1 `command-line-args` / `command-line-arg` runtime
- Related: `V2-16b`, `LEGACY-IO-01`, [`decisions-legacy-wasi-argv-split.md`](decisions-legacy-wasi-argv-split.md)

## Context

The existing standalone command-line fixture covered ordinary arguments and the
zero-argument Rust runner path, but its concatenated output could not show
whether an empty argument was preserved in the middle of argv. The native
runtime must preserve argument bytes and argc without treating spaces or UTF-8
as separators.

## Decision

- Keep `command-line-arg` byte-preserving for ordinary, empty, UTF-8, and
  whitespace-containing arguments.
- Keep `command-line-args` as the observed argc value, including empty
  elements.
- Use a delimiter-bearing fixture output so empty elements are observable:
  `prog name||雪 空|4`.
- Treat the external Wasmtime CLI's empty `--argv0` behavior as an argc=1
  boundary. Strict argc=0 remains covered by the Rust WASI runner.

## Evidence

- Rust standalone E2E: 1 passed, 336.08s.
- Mac Apple Silicon saved App.Cli artifact: native I/O matrix 16 cases passed.
- Linux x86_64 `eb8086a8` target-only artifact: native I/O matrix 16 cases
  passed as replay-only evidence.
- Python syntax compilation and `git diff --check` passed.

## Boundary

This ADR closes only argv empty-element, UTF-8, whitespace, and argc
observation for the tested standalone runtime. It does not establish
current-source Linux regeneration, all fd error/EOF combinations, dynamic
memory layout, all public commands, component packaging, release
acquisition/rollback, packaged provenance parity, or full Rust-free closure.
