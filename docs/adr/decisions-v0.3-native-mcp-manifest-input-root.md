# ADR: v0.3 native MCP manifest input root boundary

## Status

Verified partial slice (2026-08-01). Native MCP `lsharp_validate` now rejects
malformed or non-object manifest input before starting `program.native`.

## Context

The native MCP schema exposes both an inline JSON-string manifest and a
`manifest_file` route. A JSON array, `null`, number, or malformed string is
not a validation manifest, but passing it to the native program made the
failure depend on the downstream parser and could expose an unstable traceback
or consume native execution for an invalid request.

## Decision

- Parse inline `manifest` strings before writing the task-owned temporary file.
- Read and parse `manifest_file` before invoking the native program.
- Require the JSON root to be an object; reject malformed JSON and every other
  root shape as an MCP tool error.
- Keep nested schema and semantic validation in the native `validate` command;
  this preflight only establishes the root-shape boundary and does not infer
  missing manifest fields.
- Do not call `cargo`, `rustc`, host `lsharp`, a provider, or the native
  program for a rejected root.

## Evidence

`scripts/ci/native_selfhost_mcp_manifest_tests.py` covers inline array,
`null`, and numeric roots plus a `manifest_file` `null` root. The fake native
program log proves all cases are rejected before execution. The complete native
MCP suite passes with 54 tests, including the existing identity, provider,
package, LSP, and compile/run contracts.

## Remaining boundary

This is only the native MCP input root preflight. Full nested manifest runtime
validation, provider authentication/signature/lifecycle semantics, native
stage0 report parity, and current-source Linux runtime evidence remain `[~]`
under `EC-M3-05` / M3-05-N9.
