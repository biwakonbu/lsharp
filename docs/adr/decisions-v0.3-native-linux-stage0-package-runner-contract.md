# ADR: Linux x86 stage0 package runner consumer contract

- Status: Accepted (verified fake-harness contract)
- Date: 2026-08-01
- Scope: `scripts/ci/test-package-native-linux-x86-actual-stage1-vm.sh`,
  `scripts/ci/package-native-linux-x86-actual-stage1-vm.sh`, `scripts/native-selfhost-dev.sh`
- Related: `V2-16e`, `LEGACY-BOOT-01`

## Context

The actual-stage1 package wrapper already checked payload metadata and produced a stage0 package, but its
test did not prove that the package could be consumed through the public native development runner. A valid
manifest alone is not enough: the runner must use the bundled compiler transport, decoder, materializer, and
generated program without a host Rust or `lsharp` fallback.

## Decision

Extend the existing fake Lima package contract instead of starting another long native replay. The test uses
a current checkout commit, a synthetic valid Linux x86 stage1 manifest, and a fake Lima materializer. The
materialized compiler emits a two-chunk legal transport containing a tiny x86_64 write-and-return program.
The packaged stage0 is then supplied to `scripts/native-selfhost-dev.sh check`; the test requires `Int`,
`diagnostics:0`, empty stderr, a materialized `program.native`, and no invocation of blocklisted `cargo`,
`rustc`, or host `lsharp` commands.

The test supplies host-only `timeout` and `cc` shims because it runs on Mac while exercising a Linux package
contract. The actual Linux materializer and target runtime remain separate Lima gates.

## Evidence

- RED: the new runner assertion first stopped at the host `timeout` preflight, then at the missing transport
  header, then at an invalid data-tail layout, and finally at the Darwin assembler while reaching the
  package materializer.
- GREEN: `bash scripts/ci/test-package-native-linux-x86-actual-stage1-vm.sh` passed with package creation,
  runner consumption, `Int` / `diagnostics:0`, empty stderr, materialized program, and no forbidden host
  tool invocation.
- Existing package, transport, materializer frontier, native selfhost runner, release package, and docs
  contract tests remain separate focused gates.

## Boundary

This closes only the fake-harness package consumer and provenance boundary. It does not prove a current-source
Linux x86 stage1 package in the real Lima VM, source-file smoke, full stage2/stage3 fixed point, release
acquisition/rollback, or Mac/Linux runtime parity. `V2-16e` and `LEGACY-BOOT-01` remain `[~]` in `TODO.md`.
