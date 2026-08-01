# ADR: v0.4 M1-01 native report source isolation

## Status

Accepted for the native-stage0 producer safety boundary (2026-08-01,
implementation commit `bf7878926a3f937da93bf0b07744874ea54d8a22`). This ADR does
not claim an actual native stage0, Linux x86_64, Wasm runtime, or
Rust/native differential result.

## Context

The native fixture report producer passed the manifest source path directly to
the runner. The Rust compile CLI formats source before compiling and writes the
formatted bytes back to that path. A native runner sharing that behavior could
therefore mutate the checkout while producing evidence, invalidating the
source-commit provenance and making a replay non-reproducible.

## Decision

- Read source bytes and diagnostic-span coordinates from the original manifest
  fixture.
- Materialize a regular copy under the fixture's task-owned work directory and
  pass only that copy to the native runner.
- Keep artifact and runtime directories under the existing per-fixture
  isolation contract.
- Treat source immutability as a producer contract, not as a native-stage0
  completion claim.

## Evidence

Implementation commit `bf7878926a3f937da93bf0b07744874ea54d8a22` adds the copy
boundary and a regression test whose fake native runner rewrites its input
source. The test fails against the old direct-path behavior and passes after
the change. The full native producer contract suite passes 12 tests, including
fallback environment, stage0 manifest, invalid diagnostic, runtime input,
batch isolation, duplicate-ID, and source immutability checks.

The producer source is
[`semantic_fixture_native_report.py`](../../../scripts/ci/semantic_fixture_native_report.py);
the contract suite is
[`test-semantic-fixture-native-report.py`](../../../scripts/ci/test-semantic-fixture-native-report.py).
No current-source native stage0 package was reused: the available detached
Linux artifacts carry older source commits/fingerprints and are rejected as
stale by the manifest boundary.

## Consequences

Future native Mac/Linux fixture reports can run without changing the checkout,
so source-commit and evidence ownership remain auditable. Actual stage0
package/runtime, Linux VM replay, Wasm validation/runtime, and differential
evidence remain pending in V4-M1-01.
