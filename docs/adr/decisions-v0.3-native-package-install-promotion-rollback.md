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
test build exposes an atomic promotion-index failpoint only under `cfg(test)`; the native
fixture uses the explicit `LSHARP_TEST_INSTALL_FAILPOINT=promotion:<index>` test
environment input. Neither changes the normal CLI surface or external API.

The tested commit points are the final rename loop and the lockfile/module-index
metadata commit. Existing metadata is moved into the same transaction staging
area before writing. A lockfile write failure, module-index failure, or the
deterministic test failpoint restores metadata and promoted packages in reverse
order. No registry, network, or MCP route is introduced.

## Evidence

- The same local fixture contains an existing path package symlink followed by
  a fresh local Git package. A deterministic failure before promotion index 1
  causes the first promotion to be rolled back.
- GREEN requires the old path symlink target, sentinel `lock.toml`, and
  sentinel module-index to be unchanged; the fresh Git destination and
  `.install-txn-*` staging residue must be absent.
- The focused Rust and native installer suites also retain successful path,
  Git, cached-version, and module-index coverage.

The metadata fixture injects `lock` and `index` failures after both path and
fresh Git packages have been promoted. Rust keeps these failpoints under
`cfg(test)`; native accepts them only through the explicit
`LSHARP_TEST_INSTALL_FAILPOINT` test environment input. The previous lock/index
contents, existing path symlink, and module-index sentinel are restored, while
the fresh Git destination and transaction staging are removed.

This is final promotion plus metadata rollback evidence, not full installer
transactionality or filesystem durability evidence. Registry/provider/auth
acquisition, current-source Linux runtime, and Mac/Linux packaged/rollback
parity remain unverified and stay `[~]` in `TODO.md`.

## Durability boundary

The next bounded slice connects the existing Rust durable-file helper to
`lock.toml` and module-index writes. Each temporary file is `sync_all`ed before
rename and its parent directory is synced after rename; the module-index
temporary directory is synced before promotion and `.lsharp` is synced after.
Package promotion syncs the staged file/directory before rename and the final
path plus package parent after rename. Native selfhost uses `os.fsync` at the
same observable points.

Rust test-only failpoints (`promotion-before`, `promotion-after`, `lock`, and
`index`) and native `LSHARP_TEST_INSTALL_FAILPOINT` equivalents inject sync
failure without changing the normal CLI/API. The same path + fresh Git fixture
shows that every injected failure restores the prior package symlink, lock, and
module-index and removes the fresh package and transaction staging.

This verifies ordering and fail-closed recovery in an offline fixture. It does
not prove crash consistency, filesystem journaling, power-loss durability,
cross-device rename behavior, registry/provider acquisition, current-source
runtime, or Mac/Linux packaged parity; those remain `[~]`.

## Cached candidate provenance boundary

The cached-version resolver now treats every matching cache entry as an
attestable candidate before selection. Its root must be a regular directory,
the complete candidate tree must be symlink-free, and `lsharp.toml` must be a
regular, parseable manifest whose project name matches the dependency and whose
version is valid semver. Any matching invalid candidate fails closed rather
than being silently ignored; this keeps an unsafe cache from becoming a
successful install or from changing existing lock/index state.

When multiple valid candidates have the same semantic version, both Rust and
native select the lexicographically greatest cache directory name after the
version comparison. This makes the existing highest-version rule deterministic
without adding a registry or changing the lock source representation.

The same offline fixture covers valid equal-version candidates, a root
symlink, a nested symlink, and malformed manifest input. Rust resolver tests
and the native selfhost installer tests agree on the selected candidate and
on preservation of sentinel lock/index files plus transaction residue absence
when an unsafe candidate is encountered. Network/registry acquisition,
filesystem crash consistency, current-source runtime, and Mac/Linux packaged
parity remain unverified and stay `[~]`.
