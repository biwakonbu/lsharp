# ADR: Semantic artifact projection current-source admission

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Rust/native
producer parity on current Mac/Linux artifacts and target runtime evidence remain
open.

## Context

The `v4-m1-07` static projection already compared Rust-oracle and native
sidecars, but it trusted the caller-provided `source_commit`. A stale producer
could therefore project the current checkout's source file and artifact while
claiming an older checkout identity; the external `wasm-tools` producer would
run before that mismatch became visible.

This is a source-provenance admission boundary, distinct from the runtime
receipt and component-byte digest checks. It does not add fields to the
projection or semantic report schemas.

## Decision

Before reading the artifact or invoking `wasm-tools print`,
`scripts/ci/semantic_fixture_artifact_projection.py` resolves the current
checkout `HEAD` for the supplied root and requires it to equal the explicit
`--source-commit`. A stale, unreadable, or malformed current commit fails
closed with no external tool invocation and no projection sidecar. Valid
current-source projections continue to bind source digest, artifact digest,
ftable/table shape, imports, and exports for the existing Rust/native diff.

## Evidence

The same fake artifact/projection harness now covers both the positive projection
and a stale commit. The stale case uses a rejecting fake `wasm-tools`; the
current-source check fails first, proving the helper is not invoked and no
evidence file is created:

```text
python3 scripts/ci/test-semantic-fixture-artifact-projection.py
```

This is offline evidence only. It does not prove current-source Rust/native
producer byte parity, component instantiation, Mac/Linux target runtime, or
packaged/rollback parity.

## Consequences

Projection sidecars cannot silently certify a source checkout other than the
one being inspected. Existing source/ftable/import projection and report
bindings remain unchanged, while EC-M3-04 / EC-M3-05 and M3-04-N1 / M3-05-N9
remain `[~]` until current target artifacts and runtime evidence are available.
