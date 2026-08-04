# ADR: Standalone `read-file` 4MiB Bounded Chunk

- Status: Accepted as a verified partial slice
- Date: 2026-08-04
- Scope: V2-16b / LEGACY-IO-01, standalone Preview1 `read-file`

## Context

The standalone selfhost `read-file` path read 4096 bytes at a time and repeatedly
used `string-concat` to accumulate the result. A Rust focused E2E fixture with an
exactly 4MiB file did not reach its assertion after more than five minutes, so the
old path was not a practical runtime boundary for the current selfhost compiler.

The existing open/read/close failure behavior is a separate contract. This batch
must improve the bounded large-file path without changing those error semantics or
claiming arbitrary-size dynamic file support.

## Decision

Keep the existing standalone open/read/close sequence and fail-closed behavior.
Use a 4MiB file object and a 4MiB iovec for the bounded chunk, then construct the
returned String from that chunk. The emitter uses signed LEB immediates for the
allocation and read lengths, matching the existing native Wasm byte emission
conventions.

This is intentionally a narrow capacity boundary. It does not introduce a dynamic
file-size query, an unbounded file buffer, or a new fd error/EOF policy.

## Evidence

- Rust focused E2E `test_e2e_selfhost_standalone_read_file_returns_all_bytes_over_4m`: 1 passed / 316.52s.
- Mac Apple Silicon current-source gate at `ad65eaffdd7b928ec5e2d226c6f4695236afd05c`: 1 passed / 828.19s; native I/O matrix: 19 cases passed.
- Linux x86_64 current-source fixed point:
  `ci-artifacts/native-linux-x86-hostgen-vm/ad65eaff-standalone-file-over-4m/actual-selfregen-summary.json` reports target
  `x86_64-unknown-linux-gnu`, status `pass`, stage2/stage3 code length `11,448,943`, and equal stdout SHA-256
  `a66bf8c746a9cf91a6b0cdb0509a9f12b3b7987301f025646d69fdffd1c6677e`.
- Linux x86_64 `App.Cli` target-only materialize, reusing the fixed-point stage2:
  `ci-artifacts/native-linux-x86-hostgen-vm/ad65eaff-standalone-file-over-4m-linux-cli/actual-selfregen-summary.json` reports
  `selfhost_fixed_point=true`, code length `13,374,506`, program SHA-256
  `a090cd8474c6115ac3a2bcf5570226cc912d7479d0285f6991ca02fb5a1d6469`, and zero stderr bytes. The `--version` smoke returned
  `lsharp 0.1.0`.
- The Linux target-only native program passed the external Wasmtime 43.0.0 I/O matrix with 19 cases.
- Static contract, Python compilation, and `git diff --check` passed. The task-owned VM workdirs, replay lock, matrix workdir, and
  Wasmtime archive were removed, and the VM was stopped.

## Boundary

This closes only the 4MiB bounded `read-file` verified partial for Mac Apple
Silicon and Linux x86_64 native execution. Arbitrary-size dynamic file buffers,
all fd_read/fd_close/path_open error and EOF combinations, concurrent dynamic
root/data/heap layout, GC/capacity diagnostics, public command parity, component
sidecars, release acquisition/rollback, packaged provenance parity, and the
aggregate Rust-free milestone remain open in TODO.md.

## Consequences

The exact 4MiB fixture no longer falls into the old quadratic accumulation path,
and the supported native target gates have a reproducible runtime artifact. The
fixed 4MiB bound remains visible in the implementation and evidence, so future
work can replace it with a dynamically sized buffer without misclassifying this
partial slice as complete I/O parity.
