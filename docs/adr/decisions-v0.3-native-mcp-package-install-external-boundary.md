# ADR: v0.3 native MCP package-install external boundary

## Context

The Rust CLI and native selfhost installer have offline package-install slices, but the
native MCP surface had no package-install contract. An MCP caller could not distinguish a
deliberately unavailable provider operation from an accidentally missing tool, and there
was no Rust/native fixture proving that an install request cannot mutate project metadata
or invoke the native compiler.

## Decision

Advertise `lsharp_install` with the same closed input shape in Rust and native MCP:
non-empty `name` is required and `project_dir` is optional. The route validates its local
arguments, then returns the stable error
`native MCP package installation requires an explicit external provider adapter`.
It does not call the native program, registry, network, auth provider, or installer, and it
does not modify `lsharp.toml`, `.lsharp/lock.toml`, `.lsharp/module-index.json`, or package
directories. Actual installation remains a caller-owned external boundary until a provider
adapter and complete transaction/runtime evidence exist.

## Evidence

- RED: native `tools/list` had no `lsharp_install` descriptor; the Rust schema had no
  closed-world input shape and the Rust route test observed `null` for
  `additionalProperties`.
- GREEN: the native MCP harness and Rust MCP module tests use the same metadata sentinel
  fixture. They verify descriptor order/schema, the exact external-boundary error, no fake
  native execution, and byte-for-byte preservation of project metadata.
- Focused batch: native MCP 103 tests and Rust MCP 92 tests pass; Rust formatting and
  Python syntax checks pass.
- This is an explicit external boundary, not live provider/auth acquisition, registry
  retrieval, native cryptographic verification, current-source Mac/Linux runtime, or
  packaged/rollback parity.

## Consequences

MCP clients now receive a named, deterministic refusal instead of an accidental missing
route, while unsupported installation cannot partially mutate local state. `EC-M3-03`,
`EC-M3-04`, `EC-M3-05`, `M3-04-N1`, and `M3-05-N9` remain `[~]` for the unverified provider,
runtime, and packaged evidence boundaries.
