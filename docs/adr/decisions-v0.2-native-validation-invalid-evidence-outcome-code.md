# ADR: v0.2 native validation invalid evidence outcome code parity

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-invalid-evidence-enum-matrix.md`

## Context

Native source validation reports an unsupported evidence outcome as typed-field code `8`.
The selfhost consumer already mapped the same enum rejection to code `8`, but no direct Rust-host
E2E locked the observable `outcome` field/value boundary.

## Decision

Keep invalid evidence outcome rejection at code `8`, preserving field `outcome` and the wire value.
The direct selfhost E2E must return `0`, `8`, `outcome`, and the bracketed offending value.

## Evidence

- RED-first contract addition: the focused E2E was added before any implementation edit.
- GREEN: the focused E2E returns `['0', '8', 'outcome', '[not-an-outcome]']`; the existing enum
  implementation already matched the native contract, so no production code change was needed.
- Existing native smoke requires code `8`, exit `1`, no report, and no manifest for the same fixture.

## Boundary and follow-up

This closes only the direct selfhost outcome-code regression contract. It does not prove packaged
current-source stage0, Mac/Linux runtime parity, or fallback exclusion. Keep EC-M2-02 aggregate `[~]`.
