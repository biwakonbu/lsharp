# ADR: v0.2 native validation invalid evidence method code parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-invalid-evidence.md`

## Context

Native source validation already defines an invalid evidence typed field as `source validation error:8`.
The selfhost Evidence consumer used an internal method error code `7`, so the same malformed `:method`
could expose different observable codes between the direct selfhost boundary and the native source contract.

## Decision

Use code `8` for invalid evidence `:method`, preserving the offending field and value. Keep this slice
limited to method; outcome, independence, and subject-kind parity remain separate contracts.

## Evidence

- RED: the new selfhost E2E returned `['0', '7', 'method', '[not-a-method]']` before the fix.
- GREEN: the same E2E now returns `['0', '8', 'method', '[not-a-method]']`.
- Existing native smoke already requires `source validation error:8`, exit `1`, no report, and no manifest
  for the invalid method fixture.

## Boundary and follow-up

This closes only the invalid method code boundary. It does not prove packaged current-source stage0,
Mac/Linux runtime parity, or fallback exclusion. Keep EC-M2-02 aggregate `[~]`; handle other invalid
evidence fields in their own parity slices.
