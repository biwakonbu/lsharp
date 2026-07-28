# ADR: v0.2 native validation invalid evidence independence code parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-invalid-evidence.md`

## Context

The native source validation contract exposes invalid evidence typed fields as code `8`. The selfhost
Evidence consumer still used code `9` for invalid `:independence`, creating an observable mismatch for
the same source fixture.

## Decision

Use code `8` for invalid evidence `:independence`, preserving the offending field and value. Keep the
slice limited to independence; other invalid evidence fields remain separate parity contracts.

## Evidence

- RED: the selfhost E2E returned `['0', '9', 'independence', '[not-an-independence]']` before the fix.
- GREEN: the same E2E now returns `['0', '8', 'independence', '[not-an-independence]']`.
- Existing native smoke requires code `8`, exit `1`, no report, and no manifest for this fixture.

## Boundary and follow-up

This closes only the invalid independence code boundary. It does not prove packaged current-source
stage0, Mac/Linux runtime parity, or fallback exclusion. Keep EC-M2-02 aggregate `[~]`.
