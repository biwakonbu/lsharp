# ADR: v0.3 packaged LSP version output parity

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: non-native `release-smoke.sh` の packaged `lsharp-lsp --version`
- Related: [`decisions-v0.3-packaged-rollback-version-output-parity.md`](decisions-v0.3-packaged-rollback-version-output-parity.md)

## Context

The packaged release smoke executed `lsharp-lsp --version`, but discarded the
output. An archive could therefore declare one release `VERSION` while its LSP
binary reported another version; checksums and the rollback manifest could
still be valid.

## Decision

The non-native packaged smoke now compares `lsharp-lsp --version` with the
caller-provided archive `VERSION` using the same `lsharp <version>` wire format
already used for the packaged CLI. A mismatch fails closed with
`packaged LSP version mismatch`. The native CLI and rollback executable version
contracts remain separate and unchanged.

## Evidence

- RED: `bash scripts/ci/test-release-smoke-provider-snapshots.sh` accepted a
  fake packaged archive whose `lsharp-lsp` reported `9.9.9` while `VERSION` was
  `v0.0.0-test`.
- GREEN: the same focused harness rejected the mismatch and passed its complete
  provider/archive smoke suite.
- `bash -n scripts/ci/release-smoke.sh` and `git diff --check` passed.

## Boundary and follow-up

This is an offline packaged LSP version boundary only. It does not prove live
provider/auth acquisition, provider semantic verification, current-source
Linux runtime, or Mac/Linux packaged and rollback bytes parity. Those remain
`[~]` in `TODO.md` and the v0.3 planning document. No stage regeneration,
full build, or Linux VM replay was run because the current HEAD has no matching
manifest/expected replay lock and another session owns the Lima/QEMU/replayd
processes.
