# ADR: native package installer path input fail-closed parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `crates/lsharp-driver/src/main.rs`, `scripts/native-selfhost-install.py`
  and their focused installer fixtures
- Related: EC-M3-05, package-install provider boundary

## Context

The Rust `cmd_install` path dependency branch treated a missing directory or a
directory without `lsharp.toml` as a warning. It then completed successfully and
could write an empty `lock.toml`/`module-index`. The native selfhost installer
already rejected the same declared input while resolving `path_source`. Thus a
malformed local provider declaration could produce a successful but incomplete
install on Rust and a failed install on native.

This is separate from cached registry candidate provenance: it validates the
declared local provider before cache selection or package promotion and does not
add registry/network access.

## Decision

- Rust and native preflight every declared `path` dependency before creating
  managed `.lsharp` installation directories.
- The input must resolve to a directory containing a file `lsharp.toml`.
  Missing paths, non-directories, and missing/non-file manifests fail closed with
  the corresponding `path dependency ...` diagnostic family.
- Invalid path input does not create package destinations, `lock.toml`,
  `module-index`, or transaction staging. Existing managed state is therefore
  not replaced by an empty successful install.
- Valid path dependencies retain the existing symlink promotion, module-index,
  lock, durability, and rollback semantics. No new MCP install route, registry
  client, network helper, or fallback is introduced.

## Evidence

- RED: the Rust `test_cmd_install_path_dependency_missing_path` fixture showed a
  missing path being warned and skipped, followed by successful empty lock
  generation; native `path_source` rejected it.
- GREEN: Rust path input tests cover missing path, regular file, and missing
  manifest; native uses the same three cases and asserts no host fallback and no
  `.lsharp` creation. The valid path, Git, cached candidate, promotion rollback,
  metadata rollback, and sync rollback tests remain green.
- Focused batch: `cargo test -p lsharp-driver test_cmd_install -- --nocapture`
  (22 passed), `python3 scripts/ci/test-native-selfhost-install.py` (18 passed),
  `rustfmt --edition 2024 --check` for the changed Rust files, and Python
  `py_compile` for the changed installer files.
- `bash scripts/audit_docs.sh` and `git diff --check` are required before
  commit/push; no Linux replay or stage regeneration was run because the current
  source manifest/expected replay lock was absent and Lima/QEMU/replayd were
  owned by another session.

## Boundary and follow-up

This closes only local path-provider input validation and its Rust/native
fail-closed mutation boundary. Live registry/provider acquisition, crash or
power-loss filesystem semantics, full package-install transactionality, native
MCP package-install semantics, current-source Linux runtime, and Mac/Linux
packaged/rollback parity remain unverified and stay `[~]` in the active backlog.
