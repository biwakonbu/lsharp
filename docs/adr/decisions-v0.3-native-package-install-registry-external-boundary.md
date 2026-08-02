# ADR: package installer registry acquisition external boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: Rust `cmd_install`, Rust cached resolver, and native selfhost installer
- Related: EC-M3-05, package-install provider boundary

## Context

Version dependencies are currently resolved only from an explicit local
`.lsharp/packages` cache. A cache miss was reported merely as “no cached
package”; both installers created managed install directories before reaching
that failure. That left the live registry/provider boundary implicit and made a
no-cache project observe install-side filesystem mutation even though no
provider acquisition was available.

This is distinct from cached candidate provenance and local `path` validation:
valid cache selection remains supported, while live registry/network retrieval
is not introduced by this slice.

## Decision

- Rust and native preflight every version dependency before creating managed
  `.lsharp` state.
- A version dependency is accepted only when a matching, already validated
  offline cache candidate exists.
- A cache miss fails closed with the stable diagnostic family
  `registry provider acquisition is an external boundary; ... no offline cached
  semver candidate`.
- A no-cache failure creates no `.lsharp`, package destination, `lock.toml`,
  `module-index`, or transaction staging. Existing valid cache installs and
  their promotion/rollback/durability behavior remain unchanged.
- No registry client, network helper, auth flow, MCP route, or implicit host
  fallback is added.

## Evidence

- RED: the same no-cache `math-core = "1.0.0"` fixture reached a generic cache
  miss and created `.lsharp` before failing in both implementations.
- GREEN: Rust and native reject the fixture with the explicit external-boundary
  diagnostic and leave `.lsharp` absent.
- Focused batch: `cargo test -p lsharp-driver test_cmd_install -- --nocapture`
  (23 passed) and `python3 scripts/ci/test-native-selfhost-install.py` (19
  passed). This batch also covers valid cached selection, path/Git install,
  promotion rollback, metadata rollback, and sync-failure rollback.
- Rust formatting, Python syntax, docs audit, and staged diff checks are run as
  the final focused/docs gate. Linux replay, stage regeneration, and full build
  are not evidence for this slice because current-source manifest/expected lock
  is absent and Lima/QEMU/replayd are owned by another session.

## Boundary and follow-up

This verifies only the offline registry/provider acquisition boundary and its
no-mutation behavior. Live registry retrieval/authentication, complete package
transactionality, crash/power-loss filesystem semantics, native MCP
package-install semantics, current-source Linux runtime, and Mac/Linux
packaged/rollback parity remain unverified and stay `[~]`.
