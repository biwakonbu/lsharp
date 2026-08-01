# ADR: v0.4 M1-01 semantic fixture matrix contract

## Status

Accepted for the verified inventory slice (2026-08-01). This ADR does not
complete V4-M1-01 or any v0.3/legacy item.

## Context

The next-version plan needs one fixture set that can later be sent through the
Rust oracle, Rust-host selfhost, Mac native stage0, and Linux native stage0.
Existing tests cover many individual layers, but there was no small,
machine-readable inventory that made target scope, failure boundaries, or
pending artifact/runtime evidence explicit. A manifest that silently omitted a
target or accepted an implicit fallback would make a later differential report
look stronger than its evidence.

## Decision

- Use `scripts/ci/semantic-fixture-matrix.json` as the version-1 inventory for
  V4-M1-01. Every fixture names a project-relative, regular non-symlink `.ls`
  source, valid/invalid kind, covered layers, observable outputs, commands,
  expected diagnostics and spans, exit code, artifact state, and runtime state.
- Require both supported targets (`aarch64-apple-darwin` and
  `x86_64-unknown-linux-gnu`) on the manifest and every fixture in a stable
  order. Unknown or omitted targets fail closed.
- Require the execution policy `current-source`, `fallback=forbidden`, and
  `network=forbidden` at both manifest and fixture scope. The inventory cannot
  claim evidence for stale stage0 or implicit host/provider execution.
- Require every fixture whose expected artifact is required to name `compile`
  or `build` in its command list. A `check`-only fixture cannot claim an
  artifact or runtime boundary.
- Keep the scope inventory aligned with execution claims: a required artifact
  names the `codegen` layer and `wasm` observable, while expected runtime names
  the `runtime` layer and `runtime` observable. Invalid intermediate failures
  may stop at codegen without claiming an artifact.
- Keep artifact and target/runtime results explicitly `pending` or `not-run`
  until the corresponding Rust differential, native, Wasm, and target gates
  produce evidence. The validator must not promote pending data to success.
- Compare one `rust-oracle` report with one `native-stage0` report per target
  using `scripts/ci/semantic_fixture_diff.py`. Equal diagnostics/exit/runtime/
  artifact observations are `pass`; any mismatch is `mismatch`; an unobserved
  artifact or runtime boundary is `pending` with exit code 2.
- Start the Rust lane with `scripts/ci/semantic_fixture_rust_report.py` for a
  valid, no-diagnostic fixture or an invalid fixture whose Rust diagnostic
  explicitly contains both an `LS####` code and a byte span. The caller must
  provide absolute compiler and Wasmtime paths, source commit, target, and work
  directory; repeatable `--fixture-id` values select a deterministic,
  lexicographically sorted batch, with each fixture isolated under a numbered
  work directory for multi-fixture runs (the single-fixture artifact path stays
  compatible) and duplicate IDs rejected. The producer sets
  `LSHARP_DISABLE_EMBEDDED_COMPONENT=1` and never discovers a fallback or
  network provider. It converts explicit byte spans to one-based line/column
  points, but refuses to synthesize a report when a diagnostic code or span is
  missing. Native producer, full invalid coverage, and target/runtime execution
  remain explicit follow-up boundaries.
- Add `scripts/ci/semantic_fixture_native_report.py` as the native-stage0 lane.
  It requires an explicit stage0 manifest, native runner, Wasmtime executable,
  source commit, target, and task-owned work directory. Repeatable
  `--fixture-id` values use the same deterministic batch and numbered work
  directory contract as the Rust lane (while preserving the single-fixture
  artifact path). The manifest kind, target, source
  commit, and safe relative executable paths are checked before execution. The
  runner environment does not inherit `LSHARP_PATH` or the embedded-component
  disable flag, so the native runner owns its explicit stage0 boundary rather
  than silently delegating to a host compiler. Invalid output is reported only
  when an `LS####` code and source byte span are both present; missing fields
  and duplicate IDs fail closed.
- Validate the contract with the standalone Python helper and focused unittest;
  the matrix projector emits the deterministic input and the diff helper emits
  a deterministic comparison result.
- Execute the Rust-oracle lane against the current source before treating a
  fixture expectation as evidence. The Mac valid-fixture batch, including the
  corrected `valid/module-import` output, is recorded in
  [`Mac Rust-oracle valid batch ADR`](decisions-v0.4-m1-01-rust-oracle-valid-batch.md);
  native-stage0, Linux, full invalid, and differential evidence remain pending.

## Consequences

- Future runners can consume one stable fixture inventory without discovering
  target scope or expected failure semantics ad hoc.
- The current slice is useful immediately for RED/GREEN schema work, while its
  incomplete execution evidence remains visible as `[~]` in the milestone.
- Adding a fixture requires an explicit source path and observable contract;
  this intentionally adds a small amount of inventory maintenance before
  runtime parity work.

## Evidence

- `python3 scripts/ci/test-semantic-fixture-matrix.py` — focused contract tests.
- `python3 scripts/ci/test-semantic-fixture-diff.py` — pending/pass/mismatch and
  stale source/target contract tests.
- `python3 scripts/ci/test-semantic-fixture-rust-report.py` — explicit compiler/
  Wasmtime paths, artifact digest, runtime output, fallback guard, invalid
  code/span conversion, missing-diagnostic-field refusal, sorted batch,
  per-fixture isolation, and duplicate-ID rejection tests.
- `python3 scripts/ci/test-semantic-fixture-native-report.py` — stage0 manifest
  provenance, explicit native runner/Wasmtime paths, fallback environment guard,
  artifact/runtime observation, invalid code/span conversion, missing-field
  refusal, sorted batch, per-fixture isolation, and duplicate-ID rejection tests.
- `python3 scripts/ci/semantic_fixture_matrix.py --manifest scripts/ci/semantic-fixture-matrix.json --root .`
  — deterministic manifest projection.
- `python3 scripts/ci/semantic_fixture_rust_report.py` with source commit
  `ed72cb59987dfb8523886f775ab9170ecc436cc6` and target
  `aarch64-apple-darwin` — 14 valid artifact/runtime observations plus the
  explicit `LS3001` invalid observation; details and hashes are in the batch
  ADR.
- A second current-source Rust-oracle run at
  `3b6039fcd3f91e5d5c266aaeaa2f87af7c349948` observed two additional invalid
  fixtures (`LS3001` and `LS1001`) and kept three code/span-missing diagnostics
  rejected; the classification is recorded in the
  [`Mac Rust-oracle invalid batch ADR`](decisions-v0.4-m1-01-rust-oracle-invalid-batch.md).
- The diagnostic-parity implementation at current source commit
  `6943f488a213e63b5612eeabefe106357c922427` was followed by a Mac
  `aarch64-apple-darwin` Rust-oracle run over all five invalid fixtures. It
  observed `LS0001` (line 1 columns 1–2), `LS3102` (line 1 columns 1–23),
  `LS0102` (line 1 columns 1–14), `LS3001` (line 8 columns 19–21), and
  `LS1001` (line 1 columns 16–29), each with exit `1`, no artifact, and no
  runtime execution. The implementation and evidence are recorded in the
  [`Rust-oracle invalid diagnostic parity ADR`](decisions-v0.4-m1-01-rust-oracle-invalid-diagnostic-parity.md);
  native stage0, Linux, differential, and aggregate gates remain pending.
- The current-source full batch at commit
  `8af9af3c30b8260700ca6b7b05030a56c42805e3` reran all 19 manifest fixtures in
  one Mac `aarch64-apple-darwin` Rust-oracle report. Fourteen valid fixtures
  produced expected Wasm/runtime observations and passed `wasm-tools validate`;
  the five invalid fixtures produced the expected code/span/exit/no-artifact
  observations. This unified run supersedes the split valid/invalid source
  commits for current evidence and is recorded in the
  [`Mac Rust-oracle current-source full batch ADR`](decisions-v0.4-m1-01-rust-oracle-current-source-full-batch.md).
  Native stage0, Linux, differential, and aggregate gates remain pending.
- The native-stage0 producer at implementation commit
  `bf7878926a3f937da93bf0b07744874ea54d8a22` now passes a task-owned source
  copy to the runner. Its 12-test contract suite proves a mutating runner
  cannot alter the manifest fixture, while preserving the explicit stage0
  manifest, fallback, invalid diagnostic, runtime input, and batch isolation
  boundaries. This is a producer safety slice only; no stale native artifact
  is promoted to current-source evidence.
