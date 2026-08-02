# ADR: semantic producer runner workspace ownership

- Status: Accepted (verified partial)
- Date: 2026-08-02
- Scope: Rust oracle / native stage0 semantic fixture report producers

## Context

The producers already compile a task-owned copy of each source fixture so that a compiler
cannot mutate the checkout's manifest source. Their compiler subprocesses nevertheless used
the checkout root as the current working directory. A compiler or native runner that writes a
relative path could therefore leave residue outside the task-owned fixture directory.

## Decision

Invoke both compiler subprocesses with the per-fixture task-owned `fixture_dir` as `cwd`.
Source and artifact arguments remain absolute, and Wasm validation/runtime keep their existing
work/runtime directories. Relative runner output is consequently owned by the fixture staging
directory rather than the checkout root.

## Evidence

The Rust and native RED fixtures use a fake compiler/runner that writes
`runner-relative-residue` without an absolute path. Before the change the marker was not under
the fixture work directory; after the change both focused tests pass and assert that the marker
is under the task-owned work directory while the temporary checkout root remains clean. The
combined `scripts/ci/test-v4-m1-01-contracts.sh` gate passed with 19 Rust producer tests and 21
native producer tests, plus the existing matrix, differential, evidence, aggregate, docs, and
whitespace checks.

## Boundary

This proves offline/fake producer workspace ownership only. It does not prove current-source
Mac/Linux runtime execution, full Rust/native producer parity, packaged/rollback bytes parity,
provider/auth retrieval, or the Linux replay gate. The current-source manifest and expected
replay lock do not match the current HEAD, and another session owns the Lima/QEMU/replayd
processes, so Linux replay, stage regeneration, and full build remain unexecuted.

Re-audit before resuming that gate:

```bash
ps -axo pid=,command= | rg 'lsharp-linux-x86|replayd'
find . -path './target' -prune -o -type f \( -name manifest.json -o -name '*replay*lock*' -o -name 'expected-lock*' \) -print
```
