# ADR: Linux x86 string-concat bounded heap allocation

- Date: 2026-08-02
- Status: accepted for the verified slice
- Related: `V2-16e`, `V2-13a-5`, `LEGACY-BOOT-01`

## Context

The Linux x86 selfhost `string-concat` helper allocated every result with the
`mmap` syscall. The current-source stage2 replay already reaches a memory
pressure boundary, so this per-allocation mapping was a concrete working-set
risk. The existing helper ABI must remain stable: tagged/static String inputs,
length headers, byte-copy order, high-bit result tagging, callee-saved
registers, and helper call offsets.

## Decision

Use the materializer-owned `r14` native heap for x86 `string-concat`. The first
eight bytes of the heap hold the bump cursor and limit. The helper aligns
`8 + lhs_length + rhs_length` to 16 bytes, allocates from the cursor, writes a
String type/length header, copies lhs then rhs payloads, and returns the tagged
object. A zero cursor falls back to the existing `8192` data frontier, and
limit overflow returns zero. The helper grows from 195 to 197 bytes; all later
x86 trailer offsets and the trailer append length are shifted by two bytes.

This change is limited to `string-concat`. `substring` and `read-file` still
require separate allocation contracts and are not declared migrated by this
ADR. The separate `int-to-string` contract is recorded in
`decisions-v0.3-native-linux-int-to-string-bounded-heap.md`.

## Evidence

RED:

- `test_native_codegen_x86_string_concat_uses_bounded_heap_cursor` detected
  the old `mov eax, 9; syscall` byte sequence.

GREEN:

- The same focused test passed after the helper replacement.
- `test_native_codegen_x86_string_slice_concat_helper_emitters_return_executable_byte_vectors`
  passed with the 197-byte helper regression.
- x86 call-site and CLI/write-file trailer offset tests passed after the
  post-concat offset synchronization.
- A minimal Linux x86_64 Lima execution initialized an `r14` heap, passed
  dynamic tagged `"ab"` and `"Z"` inputs, and returned exit code `3` from the
  generated result length. Temporary VM files were removed and the VM was
  stopped.

## Boundary

The VM run proves the helper's narrow allocation/copy/tagging ABI, not full
current-source stage2/stage3 regeneration or fixed-point parity. Linux native
stage0 source-file smoke, packaged acquisition/rollback, the remaining x86
string/read/int allocation helpers, and both-target release parity remain
`[~]` under `TODO.md`.
