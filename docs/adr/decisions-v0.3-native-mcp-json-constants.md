# ADR: v0.3 native MCP strict JSON constants

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP JSON parsing

## Context

Python's JSON decoder accepts `NaN`, `Infinity`, and `-Infinity` by default,
although they are not JSON values. The shared native MCP decoder already
rejected duplicate object keys, so accepting these constants would leave the
same input/output boundary only partially strict.

## Decision

- Reject every non-standard JSON constant through the shared decoder's
  `parse_constant` hook.
- Apply the boundary consistently to JSON-RPC requests, native reports, LSP
  responses, and package/stdlib artifacts.
- Preserve the existing relay-specific error behavior and never return
  structured content after a non-standard constant.

## Evidence

- RED: a native report containing `NaN` and a JSON-RPC id containing `Infinity`
  were accepted by Python's default decoder.
- GREEN: both are rejected with a stable `non-standard JSON constant` diagnostic.
- Full native MCP suite passes: 79 tests.
- Python compilation, docs audit, and `git diff --check` pass.

This closes only the native MCP JSON constant boundary. Provider semantics,
target runtime, and full Rust/native parity remain active `[~]` boundaries in
`TODO.md`.
