# ADR: v0.3 native package installer mixed dependency transaction boundary

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / Rust `cmd_install` and native selfhost installer

## Context

Path dependencies were previously promoted to their managed package
destination while the dependency map was still being resolved. If a later
cached-version dependency had no matching local package, the command returned
an error but left the earlier path package in `.lsharp/packages`. The existing
lockfile and module-index happened not to be rewritten, so the package tree and
metadata could describe different install attempts.

## Decision

Use one task-owned `.install-txn-*` staging directory for dependency resolution.
Rust sorts dependency names before processing to match the native installer.
Path dependencies and fresh Git clones are materialized inside staging;
cached-version dependencies only resolve existing cache entries. Final package
promotion, lockfile generation, and module-index rebuild occur only after all
dependency resolution succeeds. A Rust Drop guard and native `finally` remove
staging on failure, while existing valid installations are not replaced during
the failed resolution phase.

This slice covers the dependency-resolution failure boundary (including the
mixed path + cached fixture). It does not claim rollback of a failure during a
later final rename, lockfile write, or module-index I/O; that remains the next
transactionality boundary. No registry, network, or MCP route is introduced.

## Evidence

- RED: the same project fixture with `a-local-lib = { path = "../local-lib" }`
  followed by missing `z-missing = "1.0.0"` promoted the path package in both
  Rust and native installers, while leaving the sentinel lock/index files.
- GREEN: both installers now reject the missing cached dependency without a
  final path destination or `.install-txn-*` residue, and preserve the
  sentinel lockfile and module-index.
- Existing path, Git, and cached-version focused tests remain green after
  staging was connected to fresh Git promotion as well.

The verified scope remains partial: final I/O rollback, registry/provider/auth
acquisition, native MCP package-install semantics, current-source Linux
runtime, and Mac/Linux packaged/rollback parity remain `[~]` in `TODO.md`.
