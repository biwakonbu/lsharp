# ADR: v0.3 native MCP package entry boundary

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP offline package projection

## Context

`.lsharp/packages` contains normal installed directories and may also contain directory symlinks created for
path dependencies. File symlinks and dangling symlinks are not installed packages. The Rust and native MCP
discovery paths previously treated any symlink entry as an installed package, which could expose fabricated
metadata or make `lsharp_package_api` inspect an arbitrary non-package path.

## Decision

- An installed package entry must satisfy `path.is_dir()`. A symlink resolving to a directory remains supported
  for path-dependency compatibility.
- Symlinks resolving to files and dangling symlinks are ignored by `lsharp_search` and project context, and are
  not eligible for `lsharp_package_api` lookup.
- Rust `mcp_context` and the native Python MCP package projection use the same directory-only predicate. The
  boundary remains offline and never invokes the native program for rejected entries.

## Evidence

- RED: `test_search_tool_ignores_non_directory_symlinks` failed in both Rust and native Python projections,
  returning file/dangling symlinks as packages.
- GREEN: the same Rust test and
  `python3 scripts/ci/test-native-selfhost-mcp.py -k search_ignores_non_directory_symlinks` pass; directory
  symlink entries remain visible and file/dangling entries are absent. Rust `lsharp_package_api` lookup for
  those entries returns the stable not-found error.
- `python3 scripts/ci/test-native-selfhost-mcp.py -k search_projects_local_packages`
- `python3 -m py_compile scripts/native_selfhost_mcp_packages.py scripts/ci/native_selfhost_mcp_package_tests.py`
- `rustfmt --edition 2024 --check crates/lsharp-driver/src/mcp_context.rs crates/lsharp-driver/src/mcp_tests.rs`

This is an offline package-discovery safety slice, not package-installation semantics, provider acquisition,
or current-source target runtime evidence. Those remain active `[~]` boundaries in `TODO.md`.
