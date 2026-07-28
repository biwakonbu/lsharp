# ADR: v0.2 native validation invalid evidence subject code parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-invalid-evidence.md`

## Context

Native source validation exposes an evidence subject with an unsupported kind as typed-field code `8`.
The selfhost Evidence consumer used code `5`, which was the wrong observable boundary for the same
`evidence:` subject fixture.

## Decision

Use code `8` for an unsupported evidence subject kind, preserving the `subject` field and wire value.
Keep malformed stable-ID shape (code `2`) and missing registered intent/claim subject (code `6`) distinct.

## Evidence

- RED: the selfhost E2E returned `['0', '5', 'subject', '[evidence:checkout/wrong-kind]']` before the fix.
- GREEN: the same E2E now returns `['0', '8', 'subject', '[evidence:checkout/wrong-kind]']`.
- Existing native smoke requires code `8`, exit `1`, no report, and no manifest for this fixture.

## Boundary and follow-up

This closes only the unsupported subject-kind code boundary. It does not prove packaged current-source
stage0, Mac/Linux runtime parity, or fallback exclusion. Keep EC-M2-02 aggregate `[~]`.
