# ADR: v0.3 Rust/native MCP provider-auth external boundary

## Status

Verified partial slice (2026-08-02). Rust and native `lsharp_validate` now reject
live provider/auth inputs before parsing, loading snapshots, invoking a native
program, or producing a validation report.

## Context

The native MCP shim already treated provider URLs and auth material as an
explicit external boundary. Rust `lsharp_validate` did not inspect those names
at runtime: a request containing `provider_url` or an auth token could be
silently ignored and return a normal report. That made the Rust and native
surfaces disagree and allowed callers to mistake ignored live acquisition input
for a completed provider operation.

## Decision

Both implementations recognize the same six live provider/auth names:
`provider_url`, `provider_api_url`, `provider_auth_token`, `provider_token`,
`auth_token`, and `auth_context`. If any is present, validation stops before
project input loading or native execution with the exact error:

`live provider/auth acquisition is an external boundary; use explicit offline snapshots`

This slice does not add a network client, auth provider, signature verifier, or
provider adapter. Explicit offline snapshot and receipt paths remain the only
supported local inputs until those external boundaries are implemented.

## Evidence

- RED: the Rust fixture containing each live provider/auth name returned a
  successful `status: unknown` report instead of refusing the request.
- GREEN: the same six-name fixture now fails closed in Rust; the native existing
  fixture asserts the same error and no native program execution.
- Focused batch: native MCP 103 tests and Rust MCP 93 tests pass; Rust format
  and Python syntax checks pass.
- No live provider, network, cryptographic verification, Linux replay, stage
  regeneration, or target runtime was executed.

## Consequences

MCP callers cannot accidentally interpret ignored provider/auth parameters as a
successful local validation. This is an explicit external-boundary parity
slice, not live provider/auth acquisition or native cryptographic verification.
`EC-M3-01` through `EC-M3-05`, `M3-04-N1`, and `M3-05-N9` remain `[~]` for the
unverified provider, runtime, and packaged evidence boundaries.
