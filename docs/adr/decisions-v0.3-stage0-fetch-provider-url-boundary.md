# ADR: Native stage0 fetch provider URL preflight

- Status: Accepted (verified local contract)
- Date: 2026-08-02
- Scope: `scripts/fetch-stage0.sh`, `scripts/ci/test-fetch-stage0-provider-url.sh`
- Related: `M3-05-N9`, `EC-M3-05`

## Context

`STAGE0_RELEASE_BASE_URL` is the provider input used to fetch the release archive and its checksum file.
Before this slice it was passed directly to `curl`, so an insecure scheme, embedded credentials, or a
query/fragment could reach the download boundary and fail only later, after provider access had started.
The existing archive, checksum, target, source-commit, and atomic-install contracts are separate boundaries
and must remain unchanged.

## Decision

Validate the provider URL before creating the download workspace or invoking `curl`:

- permit HTTPS URLs with an explicit host;
- permit absolute local `file://` URLs (including `localhost`) for controlled local release fixtures;
- reject other schemes, embedded user/password credentials, and query or fragment components with explicit
  diagnostics.

The URL preflight does not replace release checksum verification or the package manifest target/source-commit
checks. It only prevents an unsafe provider input from reaching the network/file download boundary.

## Evidence

- RED: `bash scripts/ci/test-fetch-stage0-provider-url.sh` initially reached the fake `curl` and then failed
  while reading checksums, without a provider URL diagnostic.
- GREEN: the same harness now rejects insecure scheme, embedded credentials, and query-bearing URLs before
  fake `curl` is invoked. Existing local `file://` archive tests continue to cover valid fetch/install.
- `bash -n scripts/fetch-stage0.sh scripts/ci/test-fetch-stage0-provider-url.sh` and `git diff --check`
  passed.

## Boundary

This closes only the local provider URL input boundary. It does not prove live provider API/authentication,
release credentials, current-source Linux runtime, packaged target parity, or rollback archive parity.
Those M3-05-N9 boundaries remain `[~]` in `TODO.md`.
