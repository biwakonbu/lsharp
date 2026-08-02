# ADR: v0.3 stage0 archive round-trip provenance

- Status: Accepted (verified partial slice)
- Date: 2026-08-02
- Scope: `scripts/ci/package-native-stage0-release.sh`, `scripts/ci/test-native-stage0-release-package.sh`
- Related: `M3-04-N1`, `M3-05-N2`, `M3-05-N7`, `M3-05-N9`

## Context

The stage0 release packager validated the input directory and then moved the generated tar archive to
the output directory. That proved the pre-archive tree, but did not prove that the published archive still
contained the same source-bound manifest, executable payload, evidence identity, and checksums. A producer
or archive tool that changed the tar payload after the preflight could therefore publish a provenance-invalid
stage0 archive.

## Decision

Before reporting success, `package-native-stage0-release.sh` extracts the generated archive into a task-owned
temporary directory and validates the extracted package as a regular, symlink-free stage0 package with the
requested target and source commit. It then recomputes `checksums.txt` and compares the complete extracted tree
with the pre-archive package tree. Any extraction, manifest, checksum, or tree mismatch removes the output
archive and fails with the stable `archive round-trip provenance validation failed` boundary.

This is an offline package/archive binding contract. It does not add provider acquisition, cryptographic
verification, lifecycle fields, or a new manifest schema, and it does not treat the package as runtime evidence.

## Evidence

- RED: a fake `tar` changed the archive manifest source commit after creation; the previous packager published
  the tampered archive and the focused package harness failed.
- GREEN: the same fake archive is rejected before publication, the invalid output is removed, and the focused
  package harness reports `native stage0 release package tests: OK`.
- The implementation revalidates archive entry safety, target/source binding, payload checksums, and full tree
  equality in one post-publication preflight.

## Boundary and follow-up

This closes only offline stage0 archive round-trip provenance. It does not prove current-source Mac/Linux
runtime execution, live provider/auth acquisition or semantic signature verification, packaged bytes parity
across targets, rollback runtime parity, or the current-source Linux replay. The related M3 items remain `[~]`.
