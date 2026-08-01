# ADR: Current-source Linux stage2 metadata prefix boundary

- Status: Accepted (verified diagnostic slice)
- Date: 2026-08-02
- Scope: `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs`,
  `scripts/ci/native-linux-x86-hostgen-vm-exec.sh`
- Related: `V2-16e`, `LEGACY-BOOT-01`, `LEGACY-COMP-01`

## Context

The previous current-source Linux diagnostic executed only metadata range `0..1`. A full stage2
replay remains an expensive memory-boundary experiment, so the next step must identify whether a
larger metadata prefix can execute without starting payload materialization or stage3.

## Decision

Generate stage1 from the current checkout, verify its source commit, and reuse that exact artifact
in the existing Lima VM. Execute only stage2 metadata range `0..8` with the existing 4 GiB free-space
gate and unchanged VM sizing. Keep the result as diagnostic evidence, not fixed-point evidence, and
stop the VM and remove task-owned temporary resources after collection.

## Evidence

- `test_e2e_native_linux_x86_host_generates_actual_selfregen_stage1_bundle_artifact` passed in
  `347.89s` for source commit `41be4f2b28a329addffd3cd4de55f075b76a9ec2`.
- Stage1 manifest: target `x86_64-unknown-linux-gnu`, code `4,393,425` bytes, data `2,757` bytes,
  entrypoint `4,390,965`, function-start length `3,409`, main function index `3,418`.
- The existing Lima `lsharp-linux-x86` VM had 16 GiB RAM and 12 GiB disk; free space was
  `7,688,196,096` bytes against the 4 GiB gate. Metadata range `0..8` returned
  `status=diagnostic`, `phase=stage2-metadata`, stdout `53,484` bytes, and stderr `0` bytes.

## Boundary

This proves current-source stage1 generation, VM materialization, and bounded stage2 metadata-prefix
execution. It does not prove payload materialization, full stage2/stage3 fixed-point, Linux native
stage0 source-file smoke, App.Cli artifact/runtime, package acquisition/release/rollback, or
Mac/Linux parity. `V2-16e`, `LEGACY-BOOT-01`, and `LEGACY-COMP-01` remain `[~]` in `TODO.md`.
