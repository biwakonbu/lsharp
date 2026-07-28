# ADR: v0.2 native validation empty evidence enum boundary

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/Evidence.ls`, `crates/lsharp-types/tests/validation_source/evidence.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_evidence_registry/validation.rs`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`docs/adr/decisions-v0.2-native-validation-invalid-evidence-enum-matrix.md`

## Context

The Rust source adapter treats `method`, `outcome`, and `independence` as enum fields. Empty values
therefore produce `InvalidEvidenceField` code `8`; only execution/provenance strings such as runner,
target, source commit, artifact digest, generator, producer, tool version, and timestamp use the
required-field code `4`.

The selfhost Evidence consumer checked all three enum slots in its required-field helper first, so an
empty enum value incorrectly returned code `4`.

## Decision

Limit `source-evidence-empty-field` to the same required execution/provenance strings as the Rust
contract. Empty enum values must flow through their enum validators and return code `8`, preserving
the enum field name and empty wire value.

## Evidence

- RED: selfhost actual Wasm returned `['0', '4', 'method', '[]']`, `['0', '4', 'outcome', '[]']`, and
  `['0', '4', 'independence', '[]']` before the helper change.
- GREEN: the three selfhost E2E cases now return code `8`; the Rust oracle matrix asserts the same
  `InvalidEvidenceField` for all empty enum values.
- Native source-file smoke requires code `8`, exit `1`, no report, and no manifest for all three
  fixtures; Linux source-file provenance tests require those fixtures to remain wired into the smoke.

## Boundary and follow-up

This closes the empty enum validation precedence boundary only. It does not prove a packaged
current-source stage0 artifact/runtime, Mac/Linux runtime parity, or fallback exclusion. Keep the
EC-M2-02 aggregate `[~]`.
