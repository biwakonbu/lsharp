# ADR: Native standalone allocator memory growth

- Status: Accepted (verified partial slice)
- Date: 2026-08-04
- Scope: standalone Preview1 string allocation and linear-memory growth
- Related: `V2-16b`, `LEGACY-IO-01`, `LEGACY-RUNTIME-01`

## Context

The standalone bump allocator could write past its initial 16 MiB linear
memory when `read-stdin` returned a 262144-byte payload. The old Mac native
artifact trapped at Wasm address `0x1000000`, so the failure was in the
allocator boundary rather than in the stdin transport or Wasmtime runtime.

## Decision

Before storing the next bump pointer, the standalone emitter compares the
aligned heap end with `memory.size << 16`. When the end exceeds the current
linear memory, it computes the required 64 KiB page count, executes
`memory.grow`, and then performs the existing bump allocation. The change is
limited to the standalone allocator path; GC, root tables, data layout, and
allocation failure policy remain separate contracts.

## Evidence

- RED: the pre-change Mac native artifact passed stdin sizes through 131072
  bytes and trapped at `0x1000000` for 262144 bytes.
- Rust focused E2E: `cargo test -p lsharp-wasm --test e2e
  selfhost_standalone_io::test_e2e_selfhost_standalone_read_stdin_runtime --
  --nocapture` — 1 passed / 317.18s.
- Mac Apple Silicon current-source stage0: source commit
  `19b01384281a8efdcc9f0b9ecddb4faeed36b113`, 1 passed / 827.33s; the
  materialized native I/O matrix passed 17 cases.
- Linux x86_64 current-source stage1 -> stage2 -> stage3: target
  `x86_64-unknown-linux-gnu`, host `Linux/x86_64`, status `pass`, code length
  `11,445,101` bytes at both stage2 and stage3, and matching stdout SHA-256
  `4ddaa27ed209bf8fce4305ea459a10ed99d308db7c1818222f5cfae38dbf44bc`.
- Linux x86_64 current-source App.Cli target export: source commit matches,
  `selfhost_fixed_point=true`, code length `13,370,664` bytes, program
  SHA-256 `25a3dd5c9ca786ac54c7f88ba1be7cccbf77589cee9cb65bf477817167af961d`,
  `--version` printed `lsharp 0.1.0`, and the native I/O matrix passed 17
  cases in the VM. Stage0 package provenance also records the same source
  commit and target.
- Static read-stdin contract, Python syntax compilation, and `git diff
  --check` passed. The Linux VM workdirs, replay lock, Wasmtime tarball, and
  matrix workdir were removed after verification; the VM was stopped.

## Boundary

This ADR closes only the tested standalone allocator growth path through a
262144-byte stdin payload on Mac Apple Silicon and Linux x86_64. It does not
close all fd error/EOF semantics, larger or concurrent heap/data/root layout,
GC integration, allocation failure diagnostics, all public commands,
component sidecars, release acquisition/rollback, packaged provenance parity,
or full Rust-free closure. `V2-16b` and `LEGACY-RUNTIME-01` remain `[~]`.
