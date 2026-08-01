# ADR: v0.3 native MCP LSP position alias boundary

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP LSP relay tools

## Context

The native MCP `lsharp_hover`, `lsharp_definition`, `lsharp_references`, and `lsharp_completion`
tools accept the LSP position field as either `character` or the compatibility alias `col`. The
implementation previously chose `character` when both fields were supplied, while `tools/list` only
advertised `character`. This allowed an ambiguous caller input to execute native LSP with a silently
discarded value and made the advertised schema disagree with the runtime boundary.

## Decision

- Advertise exactly two position alternatives: `{line, character}` or `{line, col}`.
- Reject a request containing both `character` and `col` before starting the native LSP process.
- Keep the existing `col` compatibility alias when it is supplied alone and normalize it to the LSP
  `character` position field.
- Apply the same contract to all four native LSP relay tools.

## Evidence

- RED: schema parity and both-alias requests failed before the implementation; native LSP was invoked
  for the ambiguous requests.
- GREEN: `python3 scripts/ci/test-native-selfhost-mcp.py -k lsp_` passes.
- Full native MCP suite passes: 62 tests, including all existing single-alias relay cases.
- `python3 -m py_compile scripts/native_selfhost_mcp.py scripts/native_selfhost_mcp_lsp.py scripts/ci/native_selfhost_mcp_lsp_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only the native MCP input/schema boundary. Native stage0 runtime, provider semantics, and
full Rust/native parity remain active `[~]` boundaries in `TODO.md`.
