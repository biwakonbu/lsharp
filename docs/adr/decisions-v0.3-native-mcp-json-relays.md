# ADR: v0.3 native MCP strict JSON relay surfaces

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP LSP and package/stdlib JSON relays

## Context

The core native MCP shim rejected duplicate JSON object keys, but LSP response
frames and installed package/stdlib `api.json` artifacts still used the default
decoder. Those surfaces could therefore accept a duplicate field and silently
project the last value, leaving the public MCP contract inconsistent.

## Decision

- Move duplicate-key rejection into a shared `native_selfhost_json` helper.
- Use it for native LSP response frames and package/stdlib API artifacts while
  retaining the existing TOML string/array parsing behavior.
- Translate duplicate-key failures into the existing relay-specific MCP error
  boundaries; do not execute native code or return structured content after a
  malformed artifact/response.

## Evidence

- RED: duplicate LSP `result`, package `api.json` `package`, and stdlib artifact
  keys were accepted by the default decoder.
- GREEN: all three relay surfaces reject duplicates with a stable diagnostic.
- Full native MCP suite passes: 79 tests.
- Python compilation, docs audit, and `git diff --check` pass.

This closes only the native LSP/package/stdlib duplicate-key relay boundary.
Provider semantics, target runtime, and full Rust/native parity remain active
`[~]` boundaries in `TODO.md`.
