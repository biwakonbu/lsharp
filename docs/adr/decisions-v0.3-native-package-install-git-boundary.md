# ADR: v0.3 native Git package-install boundary

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / Rust and native Git dependency installation

## Context

The native installer treated a Git dependency as a managed package only when
the existing destination was a real package directory with `lsharp.toml`.
Fresh clones were made in a task-owned temporary directory and promoted only
after the manifest check. Rust `cmd_install` instead cloned directly into the
final destination, treated files, directories without a manifest, and valid or
dangling symlinks as already installed, and converted clone failures into a
successful install with a generated lock/index.

## Decision

- Existing Git destinations are accepted only when they are non-symlink
  directories containing `lsharp.toml`.
- Regular files, directories without `lsharp.toml`, valid directory symlinks,
  and dangling symlinks are rejected with stable fail-closed diagnostics.
- Fresh Git clones use a managed `.tmp-*` destination. Clone failure,
  missing-manifest failure, or promotion failure removes only that temporary
  path and returns an error before lockfile or module-index generation.
- A valid local Git fixture remains offline evidence; remote Git/provider
  acquisition is still an external boundary.

## Evidence

The same local repository fixture was used by the Rust and native tests:

- `cargo test -p lsharp-driver test_cmd_install_git_dependency -- --nocapture`
  passed four Rust tests covering valid existing package reuse, clone failure,
  regular file/directory destinations, and valid/dangling symlinks.
- `python3 -m unittest scripts/ci/test-native-selfhost-install.py -k 'git_'`
  passed three native tests covering local branch/tag installation, clone
  failure cleanup and state preservation, and all invalid destination kinds.

The verified boundary does not claim full multi-dependency transactionality,
registry/cache parity, MCP package-install API parity, live provider/auth
acquisition, current-source Linux runtime, or Mac/Linux packaged/rollback
parity. Those remain `[~]` in `TODO.md`.
