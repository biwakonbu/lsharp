# ADR: Linux x86 read-file bounded heap allocation

- Date: 2026-08-02
- Status: accepted for the verified slice
- Related: `V2-16e`, `V2-13a-5`, `LEGACY-BOOT-01`

## Context

The Linux x86 selfhost `read-file` helper allocated a 2 MiB mapping for every
file. The helper is used while the native compiler reads source files, so the
per-file mapping increased the working set during stage regeneration. The
existing open/read/close behavior, tagged String representation, and
callee-saved ABI must remain unchanged.

## Decision

Use the materializer-owned `r14` native heap. Read the cursor at `(%r14)` and
the exclusive limit at `8(%r14)`. A zero cursor uses the existing data
frontier `8192`. Reserve `0x200010` bytes, cap the read at `0x200000` bytes,
and update the cursor only after the read succeeds. Limit failure returns zero
without changing the cursor. The helper still closes the file on every
failure path and preserves its String header, payload, high-bit tag, and
callee-saved register contract.

The generated helper grew from 207 to 208 bytes. All downstream x86 helper
offsets and rel32 call-site contracts were shifted by one byte.

## Evidence

RED:

- `test_native_codegen_x86_read_file_uses_bounded_heap_cursor` detected the
  old per-file mmap syscall sequence.

GREEN:

- The bounded cursor test, Linux syscall/close ABI checks, helper byte-vector
  regression, call-site offset tests, and 2 MiB source cap test passed.
- The host-side Linux x86 object smoke generated and executed an ELF object
  containing `read-file` and `string-length`; expected and actual exit code
  were both `7`. The summary is retained at
  `ci-artifacts/native-linux-x86-hostgen-vm/0fb8a19b-read-file-bounded/`.
- A Linux x86_64 Lima replay using the reusable stage1 payload completed
  stage2/stage3 native self-regeneration with `status=pass`. Both stages had
  code length `11,374,654` and stdout length `12,207,069`; both stdout files
  had SHA-256
  `7a837812e20e71378632bbe0a101d18c141e3304fb63562890e8ee4425a00930`, and
  both stderr files were empty. The fixed-point artifact is retained at
  `ci-artifacts/native-linux-x86-hostgen-vm/a41cf065-stage23-reuse/`.

## Boundary

This closes only the Linux x86 `read-file` bounded allocation and narrow object
runtime slice. It does not prove all file/error semantics, current-HEAD Linux
native stage0 source-file smoke, packaged acquisition/release/rollback, the
Mac Apple Silicon parity gate for this exact source, or the full
`LEGACY-BOOT-01` aggregate. Those remain `[~]` under `TODO.md`.
