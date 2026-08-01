# ADR: Native stage0 fetch package set and atomic install rollback

- Status: Accepted (verified local contract)
- Date: 2026-08-01
- Scope: `scripts/fetch-stage0.sh`, `scripts/ci/test-fetch-stage0-atomic-install.sh`
- Related: `V2-16e`, `LEGACY-BOOT-01`

## Context

`fetch-stage0.sh` validates a downloaded package in a temporary directory and moves an existing `stage0`
directory to a hidden backup before installing the new package. If the final move failed, the previous
package could remain only in that backup while the public `stage0` path was absent. The package checksum
loop also verified listed files but did not reject an additional regular payload absent from the checksum
manifest.

## Decision

Treat the package checksum manifest as a complete regular-file set and the final package move as an atomic
replacement boundary. Reject regular payloads not listed in `checksums.txt`. On final move failure, remove
the temporary package, restore the previous stage0 directory when one existed, and return the original move
failure. If moving the existing package to its backup fails, remove the empty backup/temporary directories
and leave the original stage0 path untouched.

## Evidence

- RED: `bash scripts/ci/test-fetch-stage0-atomic-install.sh` first accepted an archive with an extra
  checksum-unregistered payload, and separately injected a failure into the final `mv`; the existing
  `stage0/keep.txt` disappeared and a hidden previous backup remained.
- GREEN: the same test passed after the fix. It uses a local tar archive and real checksum/provenance
  validation, rejects the unregistered file, injects only the final move failure, and verifies previous-stage0
  restoration plus cleanup.
- `bash -n scripts/fetch-stage0.sh scripts/ci/test-fetch-stage0-atomic-install.sh` passed.

## Boundary

This closes only local fetch package-set validation and installation rollback. It does not prove GitHub
Release acquisition, a current Linux x86_64 or Mac Apple Silicon runtime from the fetched package, rollback
archive byte parity, or complete stage0 release operations. `V2-16e` and `LEGACY-BOOT-01` remain `[~]` in
`TODO.md`.
