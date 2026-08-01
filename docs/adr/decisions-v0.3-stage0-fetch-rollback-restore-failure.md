# ADR: Native stage0 fetch rollback restore failure

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/fetch-stage0.sh`, `scripts/ci/test-fetch-stage0-atomic-install.sh`
- Related: `M3-05-N9`, `EC-M3-05`, `decisions-v0.3-native-stage0-fetch-atomic-install.md`

## Context

The existing atomic-install contract covered a failed final move and the normal restoration of the previous
stage0 directory. It did not cover the next failure boundary: the final install move fails and the restoration
move also fails. In that case the previous package had been moved out of the public path, so returning only the
original error could leave `stage0/` absent while retaining an untracked hidden backup.

## Decision

When the rollback move fails, attempt a copy recovery from the hidden previous-package directory into the public
`stage0/` path. Remove the hidden backup only after copy recovery succeeds. If copy recovery also fails, retain
the backup and emit an explicit diagnostic naming its recovery path rather than claiming a clean rollback.

This extends the existing atomic-install failure boundary; it does not change archive entry, checksum, URL, or
manifest target/source-commit validation.

## Evidence

- RED: `bash scripts/ci/test-fetch-stage0-atomic-install.sh` injected both the final install move failure and
  the rollback restore move failure; the previous `stage0/keep.txt` disappeared and the hidden backup remained.
- GREEN: the same harness passed after copy recovery was added. It verifies the previous package remains at the
  public path and no hidden backup remains after the injected restore failure.
- The related package, archive provenance, provider URL, source-file, lock, and evidence-copy focused harnesses
  remain separate and pass.

## Boundary

This closes only local rollback recovery for a failed restore move. It does not prove live provider/auth,
current-source Linux runtime, packaged target parity, rollback archive byte parity, or real filesystem recovery
under a persistent I/O failure. M3-05-N9 remains `[~]` in `TODO.md`.
