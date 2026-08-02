# ADR: v0.3 Rust/native canonical observation bytes

## Status

Verified partial for the offline Rust-oracle/native-stage0 producer parity
boundary. This does not prove current-source target runtime parity.

## Context

`semantic_fixture_diff.py` already compared validated report fields one by one,
but the comparison did not expose a producer-independent serialized form. A
producer could therefore emit equivalent JSON through different object or
fixture ordering without a deterministic byte-level evidence value.

## Decision

Keep the Rust/native report input schema unchanged and derive a canonical
observation projection from the validated fields `id`, `source_sha256`,
`diagnostics`, `exit_code`, `artifact`, and `runtime`. Fixture entries are
sorted by ID and serialized as compact, sorted-key UTF-8 JSON. Producer, target,
and source commit remain provenance fields checked by the surrounding
comparison, not part of this producer-independent observation payload.

The comparison result records both canonical SHA-256 values. If the bytes
differ, comparison returns the existing `mismatch` status and adds a
`canonical_observation_bytes` mismatch before any pass is emitted. Evidence
index audit and two-target aggregate fixtures recompute the same value from the
referenced reports, so a copied or stale digest cannot promote evidence.

## Evidence

- RED: the focused diff test failed because the canonical helper and digest
  projection did not exist.
- GREEN: the same fake reports prove producer metadata and JSON/fixture order
  do not change canonical bytes, while an observation change changes the bytes
  and is fail-closed. Diff, evidence-index, and two-target aggregate tests
  recompute and preserve the digest contract.
- Focused command:
  `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v scripts/ci/test-semantic-fixture-diff.py scripts/ci/test-semantic-fixture-evidence-audit.py scripts/ci/test-semantic-fixture-evidence-aggregate.py`
- No network, provider acquisition, native cryptographic verification, Linux
  replay, stage regeneration, full build, or target runtime was run.

## Remaining boundary

Current-source Mac/Linux runtime, full native producer execution parity,
component instantiation, packaged/rollback bytes parity, live provider/auth,
and real Ed25519 verification remain unverified. EC-M3-01 through EC-M3-05 and
M3-04-N1 / M3-05-N9 remain `[~]`.
