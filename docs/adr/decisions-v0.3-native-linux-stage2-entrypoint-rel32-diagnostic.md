# ADR: Current-source Linux stage2 entrypoint rel32 diagnostic

- Status: Accepted (verified diagnostic slice)
- Date: 2026-08-02
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`,
  `scripts/ci/native-linux-x86-hostgen-vm-exec.sh`
- Related: `V2-13a-5`, `V2-16e`, `LEGACY-COMP-01`

## Context

The Linux x86_64 full stage2 replay remains a high-memory boundary. Before changing the x86 call
bundle, the entrypoint's generated IR call rows need to be compared with their emitted bytes and
rel32 targets. The stage1 metadata-only diagnostic can execute the entrypoint function generation
without starting the full stage2 transport.

## Decision

Use the current-source stage1 artifact and execute only metadata range `3408..3409`, which maps to
actual entrypoint function index `3418` with the ten-function import prefix. Use prefix limit `128`.
Decode each opcode `40` row's first `e8` call and compare the signed rel32 target with the expected
function-relative target printed by the diagnostic. Do not apply a call-bundle fix when the values
already agree.

## Evidence

- Stage1 source commit: `259bbca0acb8a3b432cfb835e572f8d07cf3fa36`; target:
  `x86_64-unknown-linux-gnu`; code `4,393,425` bytes; data `2,757` bytes; entrypoint
  `4,390,965`; function-start length `3,409`; main function index `3,418`.
- VM free space was `7,686,959,104` bytes against the unchanged 4 GiB gate. Metadata-only summary was
  `status=diagnostic`, `phase=stage2-metadata`, stdout `3,228` bytes, stderr `0` bytes.
- The entrypoint rows matched as follows:

  | IR idx | operand | offset | bytes | signed rel32 | target |
  | ---: | ---: | ---: | --- | ---: | ---: |
  | 0 | 3416 | 11 | `e8 dd 9e ff ff` | `-24867` | `-24851` |
  | 4 | 3417 | 49 | `e8 cd a8 ff ff` | `-22323` | `-22269` |
  | 6 | 3415 | 59 | `e8 24 8b ff ff` | `-29916` | `-29852` |

- The reusable host-side decoder is
  `scripts/ci/diagnose-native-linux-x86-entrypoint-metadata.py`. Its positive and one-byte-mutated
  negative fixture contract is covered by
  `scripts/ci/test-native-linux-x86-entrypoint-metadata-diagnostic.sh`; a mismatch exits non-zero.

## Boundary

The opcode, emitted bytes, signed rel32, and expected function-relative target agree for the three
entrypoint user calls. This does not prove that a full stage2 artifact exists, that the targets land
in a materialized stage2 bundle, or that stage2/stage3 fixed-point and runtime execution pass. It
does not justify a broad append helper, spill floor, offset-depth refactor, or minimal call-bundle
change yet. `V2-13a-5`, `V2-16e`, and `LEGACY-COMP-01` remain `[~]` in `TODO.md`.
