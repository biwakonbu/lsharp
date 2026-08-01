# ADR: stage0 fetch archive entry provenance

- Date: 2026-08-02
- Status: Accepted (verified partial slice)
- Scope: `M3-05-N9` / `EC-M3-05` / `scripts/fetch-stage0.sh`

## Context

`fetch-stage0.sh` validated archive paths and rejected symlink, hardlink, and
device entries before extraction. Other tar entry types were not classified at
that boundary, so an unknown entry could pass archive validation and reach
extraction or installation before failing, or be accepted by a platform tar
implementation.

The fetched stage0 package is a provenance input to the native source-file
smoke. Its archive boundary must accept only directories and regular files
before any extraction, checksum walk, or installation occurs.

## Decision

`validate_archive` now rejects every member that is neither a directory nor a
regular file. This retains the existing path-root, traversal, symlink,
hardlink, and device checks while covering unknown and unsupported tar entry
types with the same explicit `unsafe native stage0 archive entry type` error.

The focused local-release harness creates a valid package and an archive with
an unknown tar entry type. The valid package must fetch and install; the unsafe
archive must fail before creating its destination directory.

## Evidence

- RED: `bash scripts/ci/test-fetch-stage0-archive-provenance.sh` accepted the
  unknown entry and completed installation before the preflight was tightened.
- GREEN: the same harness passes for a valid local package and rejects the
  unknown entry with the explicit archive-entry diagnostic and no destination.
- Related stage0 package and release-package harnesses remain green.
- No real Linux runtime replay was started. The current Cloud checkout
  `e7c7e8644429acae2823af6695c7b4a842761588` has no matching current-source
  Linux stage0 artifact; the available manifests were for other source commits,
  and no active hostgen lock was present.
- Reproduction audit: `git rev-parse --verify HEAD` followed by
  `find /tmp /Users/biwakonbu/github/tmp -maxdepth 4 -type f -name manifest.json -path '*lsharp*'`.
  The result contains no manifest with the current source commit, so the Linux
  smoke cannot be invoked with a provenance-valid `LSHARP_NATIVE_LINUX_X86_STAGE0_DIR`.

## Boundary

This closes only the fetched archive entry-type provenance boundary. Current
source Linux x86_64 runtime, packaged stage0 runtime, provider/auth, rollback,
and Mac/Linux runtime parity remain unverified; `M3-05-N9` and `EC-M3-05` stay
`[~]`.
