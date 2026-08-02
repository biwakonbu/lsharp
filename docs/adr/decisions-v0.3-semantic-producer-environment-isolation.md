# ADR: semantic producer environment isolation

- Status: Accepted (verified partial)
- Date: 2026-08-02
- Scope: Rust oracle / native stage0 semantic fixture report subprocesses

## Context

The semantic report producers invoke an explicitly supplied compiler, native runner, Wasm
validator, and runtime. Inheriting the caller's `LSHARP_*` environment could nevertheless let
ambient compiler delegation, provider input, or test failpoints change the observed producer
boundary. Removing only `LSHARP_PATH` was insufficient: other `LSHARP_*` values remained visible
to child processes.

## Decision

Build each producer's child environment by copying only non-`LSHARP_*` variables. The Rust oracle
then adds its one explicit bootstrap guard,
`LSHARP_DISABLE_EMBEDDED_COMPONENT=1`; the native stage0 producer adds no `LSHARP_*` variable.
The explicit executable paths, fixture cwd, and runtime directories remain the caller-owned
inputs. This isolates provider/fallback configuration without adding a live provider or network
dependency.

## Evidence

The RED fixtures injected `LSHARP_PATH`, `LSHARP_PROVIDER_URL`, and
`LSHARP_TEST_INSTALL_FAILPOINT` into the parent environment and recorded all `LSHARP_*` values
seen by the fake compiler/runner. Before the change they leaked. After the change Rust sees only
the explicit embedded-component disable guard and native sees none. The combined
`scripts/ci/test-v4-m1-01-contracts.sh` gate passed with 20 Rust producer tests and 23 native
producer tests, plus fixture matrix, Rust/native differential, evidence, aggregate, docs, and
whitespace checks.

## Boundary

This proves offline subprocess environment isolation only. It does not implement live provider or
auth retrieval, cryptographic verification, current-source Mac/Linux runtime, full native
producer parity, packaged/rollback bytes parity, or Linux replay. The current-source manifest
and expected replay lock do not match the current HEAD, and another session owns
Lima/QEMU/replayd, so those heavy gates remain unexecuted.

Re-audit before resuming the target gate:

```bash
ps -axo pid=,command= | rg 'lsharp-linux-x86|replayd'
find . -path './target' -prune -o -type f \( -name manifest.json -o -name '*replay*lock*' -o -name 'expected-lock*' \) -print
```
