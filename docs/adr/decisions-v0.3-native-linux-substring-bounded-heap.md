# ADR: Linux x86 substring bounded heap allocation

- Date: 2026-08-02
- Status: accepted for the verified slice
- Related: `V2-16e`, `V2-13a-5`, `LEGACY-BOOT-01`

## Context

The Linux x86 selfhost `substring` helper allocated the result through the
per-allocation mmap path. Its existing contract must remain stable: signed
range handling, `end - start` length calculation, String header, payload copy,
high-bit tag, and the callee-saved register ABI.

## Decision

Use the materializer-owned `r14` native heap. Read the cursor at `(%r14)` and
the exclusive limit at `8(%r14)`. A zero cursor uses the existing data frontier
`8192`. Align `8 + (end - start)` to 16 bytes, check the allocation end before
updating the cursor, and return zero without changing the cursor on overflow.
The result address is `r14 + cursor`; the existing header, payload copy, and
tagging behavior remain unchanged. The helper grew from 145 to 147 bytes, so
all downstream x86 trailer offsets and rel32 contracts were shifted by two.

## Evidence

RED:

- `test_native_codegen_x86_substring_uses_bounded_heap_cursor` detected the
  old mmap syscall byte sequence.

GREEN:

- The no-mmap test and `end - start` length regression passed.
- The 147-byte helper emitter, substring/concat call-site bytes, trailer
  offsets, and function metadata target contracts were synchronized and passed
  their focused tests.
- The implementation is included in the current host-side Linux x86 object
  smoke matrix; current VM object-runtime evidence is tracked separately.

## Boundary

This closes only the Linux x86 substring bounded allocation/copy/tagging slice.
It does not prove current-source VM object execution, full stage2/stage3
transport/materialization/fixed-point parity, Linux native stage0 source-file
smoke, package acquisition/release/rollback, read-file runtime evidence, or
Mac/Linux release parity. Those remain `[~]` under `TODO.md`.
