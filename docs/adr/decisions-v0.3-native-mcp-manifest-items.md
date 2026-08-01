# ADR: v0.3 native MCP manifest node and review items

- Date: 2026-08-01
- Status: Accepted (verified partial slice)
- Scope: `EC-M3-05` / native MCP emitted manifest `nodes` and `reviews`

## Context

The native MCP shim validated an emitted manifest's root, schema version, and top-level array
types, but forwarded malformed `nodes` and `reviews` items. The advertised manifest schema
already declared closed item objects, identifiers, enums, text, spans, and review provenance
fields.

## Decision

- Validate every manifest node item: closed fields, required kind/namespace/key/text, kind enum,
  identifier pattern, non-empty text, and optional non-negative integer span offsets.
- Validate the optional reviews array and each review item: closed fields, required identifiers
  and provenance digest, visibility enum, and optional verification-state enum.
- Keep manifest `evidence` and `edges` nested validation as separate active boundaries because
  their execution/subject/relationship schemas are larger and require their own fixtures.

## Evidence

- RED: malformed node and review items were accepted after emitted-manifest root validation.
- GREEN: missing/unknown fields, invalid enums, empty text, invalid spans, and malformed review
  values fail closed; a valid node/review manifest is returned unchanged.
- Full native MCP suite passes: 73 tests.
- `python3 -m py_compile scripts/native-selfhost-mcp.py scripts/ci/native_selfhost_mcp_manifest_tests.py scripts/ci/test-native-selfhost-mcp.py` passes.

This closes only emitted manifest `nodes`/`reviews` item shape. Evidence/edges nested runtime,
native stage0, provider semantics, and full Rust/native parity remain active `[~]` boundaries in
`TODO.md`.
