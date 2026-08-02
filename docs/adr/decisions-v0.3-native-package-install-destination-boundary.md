# ADR: v0.3 native package-install destination boundary

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / Rust and native path-package installation

## Context

The offline native installer already treats a path dependency as a managed
symlink under `.lsharp/packages/<name>-<source-hash>`. The Rust `cmd_install`
path branch previously removed any existing destination before creating that
symlink. A regular file or directory at the managed destination could
therefore be deleted by an install request, unlike the native installer.

## Decision

- A managed path-package destination that already exists as a regular file or
  directory is rejected with `refusing to replace non-symlink path package`.
- An existing symlink remains replaceable, but Rust creates the replacement
  symlink beside the destination and commits it with `rename`; a failed
  creation or rename removes only that task-owned temporary symlink.
- This contract is offline and covers only path-package installation. Git and
  registry acquisition remain external/provider boundaries; it does not claim
  MCP package installation or target runtime parity.

## Evidence

The same path dependency fixture creates a colliding regular package
directory with a sentinel. Before the fix, the Rust test observed successful
installation and deletion of that destination (RED). After the fix:

- `cargo test -p lsharp-driver test_cmd_install_path_dependency -- --nocapture`
  passed five Rust install tests, including collision rejection.
- `python3 scripts/ci/test-native-selfhost-install.py` passed eight native
  install tests, including the same collision and sentinel-preservation
  contract.

The native MCP package discovery/API projections remain read-only and are not
expanded with a new install route. Current-source Linux runtime, live
provider/auth acquisition and semantic verification, and Mac/Linux packaged
or rollback parity remain unverified; `EC-M3-05` and M3-05-N9 stay `[~]`.
