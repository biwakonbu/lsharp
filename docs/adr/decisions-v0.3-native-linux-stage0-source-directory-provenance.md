# ADR: Linux native stage0 source directory provenance preflight

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `M3-04-N1` / `EC-M3-04` / `scripts/ci/native-linux-x86-native-stage0-source-file-smoke.sh`

## Context

The Linux x86_64 source-file smoke accepts a stage0 directory from
`LSHARP_NATIVE_LINUX_X86_STAGE0_DIR` and copies it into Lima before running the
native source fixture. Checking only the manifest and its executable fields is
insufficient: a root directory symlink or a nested symlink can redirect the
copy input to bytes outside the intended stage0 artifact.

The source-file smoke must establish the input provenance boundary before VM
startup, copy, or guest execution. This is an operator/preflight contract; it
does not manufacture current-source Linux runtime or packaged-artifact
evidence.

## Decision

`native-linux-x86-native-stage0-source-file-smoke.sh` now requires the resolved
stage0 input to be a non-symlink directory. It then scans the directory with
`find -P` and rejects the first descendant symlink before manifest validation,
Lima startup, or any copy operation. Root and nested symlink failures report an
explicit error and exit non-zero.

The existing fake Lima harness in
`test-native-linux-x86-native-stage0-source-file-smoke.sh` covers both root and
nested symlinks and asserts that neither case invokes `limactl`. Existing replay
lock and evidence-copy harnesses remain separate regression gates.

## Evidence

- RED: the root symlink fixture was accepted by the existing wrapper and
  reached the fake Lima smoke path.
- GREEN: root and nested symlink fixtures are rejected before fake Lima
  invocation; the existing stage0 manifest/stopped-VM/cleanup cases still pass.
- Focused related harnesses:
  `test-native-linux-x86-source-smoke-replay-lock.sh` and
  `test-native-linux-x86-source-smoke-evidence-copy.sh`.
- The verification scope is limited to shell/fake-Lima contracts and docs.
  No Linux VM replay was started because the current-source stage0 runtime and
  packaged provenance boundary remain unverified and the task explicitly
  avoids duplicate heavy replay.

## Boundary

This closes only the regular-directory/no-symlink preflight for the Linux
source-file smoke input. `M3-04-N1` and `EC-M3-04` remain `[~]`: current-source
Linux x86_64 runtime, fetched/packaged provenance, provider/auth, and the
Mac/Linux runtime matrix still require separate evidence.
