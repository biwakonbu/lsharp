# ADR: v0.2 native validation project-level duplicate nodes

- Status: Accepted (verified partial slice)
- Date: 2026-08-01
- Scope: EC-M2-01 source graph duplicate-node diagnostics and native source-file smoke
- Related: `EC-M2-01`, `docs/adr/decisions-legacy-validation-source-nodes-split.md`

## Context

The source adapter must reject a typed intent ID that is declared more than once
in one project, including declarations nested under `module`, `private`, and
`impl`. A declaration-local check is insufficient: it can miss a duplicate in a
different traversal branch or produce a diagnostic that does not identify both
the first and current declaration.

## Decision

Use the shared fixture
`tests/fixtures/validation/ec-m2-project-duplicate-source.ls` as the observable
contract. Both the Rust source adapter and selfhost `IntentSource` reject the
duplicate with stable code `2`, the typed ID, the first declaration span, and the
current declaration span. The native source-file smoke copies and executes the
same fixture on both supported targets. ID omission remains fail-closed; this
ADR does not introduce an automatic naming rule.

## Evidence

- RED: existing selfhost source-adapter expectations still used duplicate code
  `4`; the shared fixture was not covered by the Rust source suite or native
  source-file smoke.
- GREEN: `cargo test -p lsharp-types --test validation_source -- --nocapture`
  passed 62 tests.
- GREEN: `cargo test -p lsharp-wasm --test e2e selfhost_intent_source_adapter
  --no-fail-fast -- --nocapture` passed 41 tests.
- Native contract tests passed for the Mac source-file smoke script and Linux
  x86 wrapper/static fixture requirements.
- Mac `aarch64-apple-darwin` current-source stage0 producer passed the actual
  App.Cli E2E in 829.28 seconds; the packaged stage0 source-file smoke passed.
- Linux `x86_64-unknown-linux-gnu` actual stage1→stage2→stage3 fixed-point passed
  with equal SHA-256 `aa5cee91b5f47dd54a7da64492859b1b9eede381059051713e85310115ba7ad`
  and equal code length `11,332,908`; the materialized package source-file smoke
  passed.

The native packages were generated from source commit `197ce48d` and removed
after verification. The checkout was then fast-forwarded through documentation
and schema-only commits to `ed72cb59`; the selfhost source tree did not change.

## Boundary and follow-up

This closes only project-level duplicate diagnostics and their two-target
source-file execution slice. Project-wide aggregate parity, manifest/MCP/public
surface parity, and the remaining EC-M2-01 completion evidence are still
required. `TODO.md` therefore keeps `EC-M2-01` as `[~]`.
