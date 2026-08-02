# ADR: v0.3 native MCP report/manifest projection parity

## Status

Verified partial slice (2026-08-02). This decision fixes the
`lsharp_validate --include-manifest` parity boundary between the native report
and its emitted manifest.

## Context

The Rust canonical implementation builds one intent graph and projects that
graph into both the validation report and the optional manifest. The native
MCP shim validates the report and separately validates the emitted manifest,
but previously replaced any report-side `manifest` without checking whether
the two valid JSON objects described the same graph. A producer could
therefore return two divergent observable surfaces and still pass postflight.

## Decision

- When a native report contains `manifest` and `--include-manifest` is active,
  the report manifest must be structurally equal to the separately emitted
  manifest.
- A valid but different report manifest is an MCP error with
  `native validate report manifest projection mismatch`; the shim fails before
  replacing the report field.
- A report without an embedded manifest remains supported for existing native
  producers; the validated emitted manifest is still attached as the final
  report projection. Receipt, provider identity, and manifest schema checks
  remain separate contracts.

## Evidence

- RED: `python3 scripts/ci/test-native-selfhost-mcp.py -k
  report_manifest_mismatch` accepted a fake native report containing a valid
  embedded manifest different from the emitted manifest.
- GREEN: the same focused test rejects the mismatch without a traceback.
- Rust differential: `cargo test -q -p lsharp-driver
  test_validate_tool_projects_valid_attestation_context_to_report_and_manifest`
  passed, confirming the canonical report/manifest projection is sourced from
  one graph.
- The focused native MCP manifest suite, Python syntax checks, docs audit, and
  diff check are run together after this change.

## Remaining boundary

Native cryptographic verification, live provider/auth acquisition,
current-source Linux runtime, and Mac/Linux packaged/rollback bytes parity
remain `[~]` in TODO/planning. Linux replay, stage regeneration, and full build
were not started because the current-head manifest/expected replay lock was
absent and the Lima/QEMU/replayd resources were owned by another session.
