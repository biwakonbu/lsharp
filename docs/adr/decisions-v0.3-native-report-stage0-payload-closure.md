# ADR: native report stage0 payload closure

- Status: Accepted (verified partial)
- Date: 2026-08-02
- Scope: native-stage0 semantic fixture report preflight

## Context

The native report producer already checked the stage0 kind, target, source commit, safe
relative paths, and compiler-to-runner identity. It did not verify that the other two required
stage0 payloads declared by the same manifest, `transport_driver` and `materializer`, existed as
regular executable files. An incomplete or symlinked stage0 could therefore reach report
production even though the stage0 package was not closed.

The ordinary `scripts/native-selfhost-dev.sh` path already treats all three declared payloads as
required regular executables. The semantic native report path must apply the same admission rule
before copying a fixture, invoking the runner, or writing evidence.

## Decision

During native stage0 manifest loading, validate `compiler`, `transport_driver`, and `materializer`
as safe relative paths whose resolved leaf is a non-symlink executable regular file. Only after
all three pass, retain the existing exact compiler-to-runner identity check and continue to the
fixture producer.

## Evidence

The RED fixture removed or symlinked each of `transport_driver` and `materializer`; the previous
producer still generated a report. The GREEN fixture rejects all four cases before runner
execution and leaves the report absent. The combined
`scripts/ci/test-v4-m1-01-contracts.sh` gate passed with 19 Rust producer tests and 22 native
producer tests, along with matrix, Rust/native differential, evidence, aggregate, docs, and
whitespace checks.

## Boundary

This verifies native stage0 manifest payload closure offline. It does not prove current-source
Mac/Linux runtime execution, full Rust/native producer parity, packaged/rollback bytes parity,
provider/auth retrieval, or Linux replay. The current-source manifest and expected replay lock do
not match the current HEAD, and another session owns Lima/QEMU/replayd, so heavy gates remain
unexecuted.

Re-audit before resuming the target gate:

```bash
ps -axo pid=,command= | rg 'lsharp-linux-x86|replayd'
find . -path './target' -prune -o -type f \( -name manifest.json -o -name '*replay*lock*' -o -name 'expected-lock*' \) -print
```
