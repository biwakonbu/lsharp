# ADR: Native standalone stdin capacity growth

- Status: Accepted (verified partial slice)
- Date: 2026-08-04
- Scope: standalone Preview1 stdin transport and String buffer growth
- Related: `V2-16b`, `LEGACY-IO-01`, `LEGACY-RUNTIME-01`

## Context

The standalone `read-stdin` path appended each fd_read chunk by allocating a
new String-sized object for the accumulated result. A payload of
`(4 * 1024 * 1024) - 1` bytes made that strategy unnecessarily expensive and
exposed the interaction between the String ABI, fd_read iovec scratch space,
and the standalone allocator. The earlier allocator `memory.grow` fix solved
the 256 KiB boundary, but did not remove the repeated buffer-copy behavior.

## Decision

The standalone emitter now starts `read-stdin` with a 4096-byte String
capacity. When the next chunk would exceed capacity, it doubles the capacity,
allocates one replacement String object, copies the existing bytes with
`memory.copy`, and continues reading into the new buffer. The object length is
stored at the existing String ABI offset, while fd_read keeps using the fixed
scratch/iovec layout. The standalone WASI call index remains the encoded
reserved index used by the existing runtime-call mapping.

This is limited to the standalone stdin buffer. fd error/EOF policy, the
allocator's broader dynamic layout, GC, and allocation-failure diagnostics are
separate contracts.

## Evidence

- RED: the 4 MiB-minus-one stdin fixture exercised the old repeated
  reallocation path and was kept as the boundary contract.
- Rust focused E2E:
  `cargo test -p lsharp-wasm --test e2e
  selfhost_standalone_io::test_e2e_selfhost_standalone_read_stdin_runtime --
  --nocapture` — 1 passed / 315.43s.
- Mac Apple Silicon current-source stage0: source commit
  `8ab2dd589410fa668ffa5c01f596bdfa046d466c`, 1 passed / 825.61s; the
  materialized native I/O matrix passed 18 cases.
- Linux x86_64 current-source stage1 -> stage2 -> stage3: target
  `x86_64-unknown-linux-gnu`, host `Linux/x86_64`, status `pass`, code length
  `11,449,265` bytes at both stage2 and stage3, matching stdout SHA-256
  `179821d0fddaceaac637b08a128beee7c31c4afdc4bd4a90e88b013755855f3d`, and
  empty stderr.
- Linux x86_64 current-source App.Cli target-only materialize:
  `selfhost_fixed_point=true`, code length `13,374,828` bytes, program
  SHA-256 `f21ebd22261a2dd392e5123293faac948dcf40c586069966cfcab46545960cc4`,
  empty stderr, and 18 native I/O matrix cases passed.
- Static read-stdin contract, Wasm validation, Python syntax compilation,
  and `git diff --check` passed. Temporary VM/matrix workdirs and the
  Wasmtime archive were removed, and the Linux VM was stopped.

## Boundary

This ADR closes only the tested standalone String capacity-growth path through
a 4 MiB-minus-one stdin payload on Mac Apple Silicon and Linux x86_64. It does
not close all fd error/EOF semantics, larger or concurrent root/data/heap
layouts, GC integration, allocation-failure diagnostics, all public commands,
component sidecars, release acquisition/rollback, packaged provenance parity,
or full Rust-free closure. `V2-16b` and `LEGACY-IO-01` remain `[~]`.
