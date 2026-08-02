# ADR: v0.3 native package installer promotion rollback

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / Rust `cmd_install` and native selfhost installer

## Context

The previous transaction slice delayed path and fresh Git promotion until
dependency resolution completed, but a failure during the final promotion loop
could still leave an earlier destination changed. Existing lockfile and
module-index writes occur after promotion, so this boundary needs to restore
managed destinations before metadata is touched.

## Decision

Before each final promotion, move an existing managed destination into the
transaction staging directory as a backup. If a later promotion fails, remove
newly promoted destinations in reverse order and restore each backup. The Rust
test build exposes an atomic-index failpoint only under `cfg(test)`; the native
fixture uses the explicit `LSHARP_TEST_INSTALL_FAILPOINT=promotion:<index>` test
environment input. Neither changes the normal CLI surface or external API.

The tested commit point is the final rename loop. Lockfile and module-index
writes remain after that loop and are not claimed as fully rollback-safe by this
slice. No registry, network, or MCP route is introduced.

## Evidence

- The same local fixture contains an existing path package symlink followed by
  a fresh local Git package. A deterministic failure before promotion index 1
  causes the first promotion to be rolled back.
- GREEN requires the old path symlink target, sentinel `lock.toml`, and
  sentinel module-index to be unchanged; the fresh Git destination and
  `.install-txn-*` staging residue must be absent.
- The focused Rust and native installer suites also retain successful path,
  Git, cached-version, and module-index coverage.

This is promotion-loop rollback evidence, not full installer transactionality.
Lockfile/module-index I/O rollback, registry/provider/auth acquisition,
current-source Linux runtime, and Mac/Linux packaged/rollback parity remain
unverified and stay `[~]` in `TODO.md`.
