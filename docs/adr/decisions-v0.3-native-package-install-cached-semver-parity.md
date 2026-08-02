# ADR: v0.3 native package installer cached semver parity

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / Rust `cmd_install` and native selfhost cached-version installer

## Context

The Rust and native installers resolve version dependencies from the local
`.lsharp/packages` cache without network access. Native parsing already requires
each semver component to contain only ASCII decimal digits, while the Rust
parser delegated directly to `u64::parse`. A signed component such as
`+1.0.0` could therefore be accepted by Rust and produce a lock entry even
though the native installer rejected the same dependency.

The broader multi-dependency transaction boundary is not folded into this
slice: earlier path or Git dependencies can still be promoted before a later
dependency fails. That boundary needs its own fixture and rollback design.

## Decision

Keep the existing three-component semver contract and make Rust validate each
component as non-empty ASCII decimal digits before numeric parsing. This keeps
the Rust cache resolver and the native resolver aligned for signed or otherwise
non-decimal version requirements and cached manifest versions. No registry,
network, MCP route, or schema field is introduced.

## Evidence

- RED: the same offline `math-core` cache fixture with dependency
  `math-core = "+1.0.0"` succeeded in Rust but failed with native
  `invalid semver`.
- GREEN: the Rust `cmd_install` fixture now rejects the requirement before
  writing a lock entry, and the native fixture continues to reject it without
  invoking Cargo or host `lsharp`.
- Focused commands:
  - `cargo test -p lsharp-driver test_cmd_install_version_dependency_rejects_signed_semver_requirement -- --nocapture`
  - `python3 scripts/ci/test-native-selfhost-install.py -k signed_cached_version_requirement`

This verifies only the offline cached-version lexical boundary. Multi-dependency
atomic promotion/rollback, registry or live provider/auth acquisition,
current-source Linux runtime, and Mac/Linux packaged/rollback parity remain
unverified and stay `[~]` in `TODO.md`.
