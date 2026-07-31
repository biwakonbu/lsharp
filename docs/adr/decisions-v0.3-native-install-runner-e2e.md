# ADR: v0.3 native selfhost install runner integration evidence

## Status

Verified partial slice (2026-08-01). The public native selfhost runner now has
an executable integration contract for `install` using the actual native
installer helper, while real current-source Mac/Linux stage0 release evidence
remains a separate gate.

## Context

`native-selfhost-dev.sh` routes `install` to
`native-selfhost-install.py` instead of invoking the generated program. The
direct installer tests already cover path, git, cached-semver, lockfile, module
index, and managed-directory safety. The missing evidence was the public
runner path itself: a stage0 manifest must be accepted, the current checkout
provenance must be checked, and the actual helper must update the project
without a Rust or host `lsharp` fallback.

## Decision

- Add a no-VM integration test that creates a task-owned fake x86_64 stage0
  package, performs source-commit provenance validation, and invokes
  `native-selfhost-dev.sh install` from a project directory.
- Copy the actual `native-selfhost-install.py` into the fixture; do not replace
  it with a routing fake. Use a local path dependency so the test has no
  network or git-provider dependency.
- Require the runner to create the installed package symlink, `.lsharp/lock.toml`,
  and exported `.lsharp/module-index/Geometry.path`. Poison `cargo` and host
  `lsharp` in `PATH` and fail if either is executed.
- Keep this as integration evidence only. It does not claim real Mac Apple
  Silicon/Linux x86_64 stage0 regeneration, packaged release bytes, or target
  runtime parity.

## Evidence

- `scripts/ci/test-native-selfhost-install-runner.sh` passes with the actual
  installer helper and a provenance-checked fake stage0.
- `scripts/ci/test-native-selfhost-install.py` continues to pass its seven
  direct installer cases.
- The test owns and removes all temporary project, package, stage0, and host
  poison artifacts; no Linux VM or shared replay lock is used.

## Remaining boundary

`V2-16c` / `LEGACY-TOOL-01` still requires real current-source stage0 and
external-tool E2E, Rust-only option/target rejection across supported targets,
and target-specific release evidence. Those remain `[~]` in `TODO.md`.
