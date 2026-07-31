# ADR: EC-M3 native validation boundary follow-ups

- Date: 2026-07-31
- Status: Accepted (verified preflight slice)
- Scope: `EC-M3-01` / `EC-M3-02` / `EC-M3-05-N9`

## Context

Mac native source-file smoke reached validation cases that were already covered by the Rust
oracle, but the selfhost parser and CLI still had boundary differences:

- review-attestation spans ended at the directive token instead of the final named field;
- missing or extra named fields were accepted as empty/default payloads;
- an unregistered `supports`/`contradicts` evidence ID was reported as an invalid wire ID first;
- positional version 1 manifest input, manifest write failure, and native stderr/exit propagation
  were not closed;
- the failed-review fixture counted a failed independent review as an independent review.

## Decision

- Compute review-attestation, evidence, and source pair/triple directive end spans after parsing the
  payload. Preserve explicit empty strings as validation errors, but append a malformed sentinel
  when required fields are missing or an extra field remains in the directive payload.
- Check evidence registry membership before evidence wire-shape validation for
  `supports`/`contradicts`, matching the existing source adapter precedence contract.
- Keep the EC-M3 native duplicate-node diagnostic code at `2`, as required by the existing smoke and
  ADR contract.
- Accept a positional version 1 manifest only when its canonical top-level markers and required
  trace relations are present. This is a fail-closed boundary parser, not a generic JSON decoder;
  unrecognized input continues through source parsing. Verify manifest writes by checking the file
  exists after the write, and route native `error:` lines to stderr while preserving the program
  exit code in the shell runner.
- Count only `method=review`, `independence=independent-review`, and `outcome=pass` as an
  independent review in the native source smoke.

## Evidence

- RED→GREEN focused contracts in
  `crates/lsharp-wasm/tests/e2e/selfhost_native_stage_chain.rs` cover parser spans, malformed
  payloads, evidence precedence, manifest input, duplicate-node code, and stderr/exit propagation.
- `cargo check -p lsharp-driver` passed after the selfhost parser changes.
- Mac Apple Silicon source-file smoke passed all validation, report, manifest, and negative cases
  with blocked `cargo`, `rustc`, and host `lsharp` tools using the existing stage0 package:
  `ci-artifacts/native-source-smoke/aarch64-apple-darwin-full-manifest-input13-64b207ed`.

The smoke artifact above is a working-tree preflight because its stage0 manifest still names
`64b207ed`. It is not current-source provenance evidence. A fresh producer and a new smoke run
after this change are required before N9 can move beyond `[~]`.

## Consequences

The native source boundary now fails closed at the same observable payload/precedence points as
the Rust contract while preserving the existing error-code taxonomy. The manifest input support is
deliberately narrow until a typed selfhost JSON decoder and full graph reconstruction are available.
Linux x86_64 runtime, packaged artifact bytes, provider input, rollback, and current-source
provenance remain open work.
