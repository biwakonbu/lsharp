# ADR: Semantic source-to-artifact projection parity

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. The full
source/ftable/import producer and target-runtime requirement remains open.

## Context

The semantic fixture report already binds an observed Wasmtime invocation to
the artifact digest, but its schema intentionally does not claim that the
source-declared `ftable`/`imports` observables are present in the artifact.
`valid/nested-record-pattern` declares those observables in the fixture matrix,
so comparing only report fields could accept a producer that emitted a runtime
result from an ABI-shape-different artifact.

The current checkout has no current-source Linux replay manifest/expected lock,
and another session owns the Lima/QEMU/replayd resources. This slice therefore
uses an explicit, offline static artifact boundary and does not claim a real
Mac/Linux runtime result.

## Decision

Add the sidecar `v4-m1-07` projection command
`scripts/ci/semantic_fixture_artifact_projection.py`. Given one matrix fixture,
one regular non-symlink Wasm artifact, the current source checkout, and an
explicit `wasm-tools` executable, it:

1. rejects missing, symlinked, empty, or non-Wasm source/artifact inputs;
2. invokes only `wasm-tools print` and never instantiates the module;
3. records source path/digest, artifact digest, ordered imports, table shape
   (the static artifact ftable shape), exports, target, and source commit; and
4. requires matrix-declared `imports` and `ftable` observables to be
   represented by non-empty artifact projections before atomically writing a
   closed sidecar JSON object.

`semantic_fixture_artifact_projection_diff.py` compares the Rust-oracle and
native-stage0 sidecars exactly. When the existing reports are supplied, it
also requires each sidecar artifact digest to match the report artifact digest
and any observed runtime artifact digest. This connects source fixture → static
artifact projection → existing runtime/evidence without adding fields to the
semantic report schema.

## Evidence

The same fake Wasm artifact and fake `wasm-tools print` harness pass the
positive projection, reject a helper failure before writing evidence, reject a
Rust/native table mismatch, and pass when both sidecars bind to matching
existing runtime reports:

```text
python3 scripts/ci/test-semantic-fixture-artifact-projection.py
```

The tested contract is offline/fake evidence only. It does not prove that the
Rust and native producers emit identical source/ftable/import semantics on
current Mac and Linux stage0 artifacts, nor that a Wasm runtime instantiates
those artifacts on both targets.

## Consequences

The artifact projection is deliberately a sidecar rather than a report-schema
field, so existing report consumers remain unchanged while the evidence gate
can bind the sidecar to them by digest. A future target/runtime gate must reuse
the same sidecar and report binding; it must not promote static projection into
runtime parity by itself.
