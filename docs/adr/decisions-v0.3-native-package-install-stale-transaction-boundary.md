# ADR: package installer stale transaction ownership boundary

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: Rust `cmd_install_in` and native selfhost installer
- Related: EC-M3-05, package-install transactionality

## Context

The installer uses a task-owned `.install-txn-*` directory while resolving and
promoting packages. A process interruption can leave such a directory behind.
The previous Rust/native paths did not share an ownership rule: Rust only
rejected the exact current-process transaction path, while native generated a
new UUID transaction and could ignore an older staging directory. Continuing
in that state could mix unknown staged data with a new promotion and obscure
which process owns rollback state.

## Decision

- Before starting a transaction, both installers scan the managed packages
  directory for any `.install-txn-*` entry.
- Any existing entry is an explicit fail-closed boundary with the stable
  diagnostic family `install transaction staging already exists; refusing to
  reuse unknown owner`.
- The installer does not remove or inspect-recover the unknown staging entry.
  Existing package destinations, `lock.toml`, `module-index`, and the stale
  entry remain byte-for-byte unchanged.
- A clean packages directory keeps the existing path/Git/cached-version
  staging, promotion, metadata rollback, and durability behavior. No registry,
  network, auth, or runtime surface is added.

## Evidence

- RED: an equivalent Rust/native fixture with `.install-txn-stale` was
  previously accepted and completed a new install, leaving unknown staging
  alongside new state.
- GREEN: both installers reject before promotion, preserve package/metadata
  sentinels and the stale owner sentinel, and do not invoke host fallback.
- Focused batch: Rust `cargo test -p lsharp-driver test_cmd_install -- --nocapture`
  (24 passed) and `python3 scripts/ci/test-native-selfhost-install.py`
  (20 passed).
- Linux replay, stage regeneration, and full build remain out of scope because
  current-source manifest/expected lock is absent and the Lima/QEMU/replayd
  resources are owned by another session.

## Boundary and follow-up

This verifies only unknown stale transaction ownership and no-mutation
preservation. It does not recover interrupted transactions or prove crash,
power-loss, journaling, cross-device rename, complete transactionality, live
registry/provider/auth, current-source Linux runtime, or Mac/Linux packaged and
rollback parity. Those remain `[~]`.
