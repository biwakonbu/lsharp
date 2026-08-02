# ADR: Semantic report batch staging transaction

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Current-source
Mac/Linux producer execution, packaged/rollback parity, and live provider/auth
remain open.

## Context

The Rust-oracle and native-stage0 semantic report producers already delayed the
report write until every selected fixture had been observed. A late compile or
runtime failure nevertheless left the task-owned per-fixture work directories
and artifacts behind. That residue could be mistaken for a partial evidence
batch and was not safe when a caller-owned sentinel shared the work root.

This is distinct from source commit admission, source fingerprint binding,
artifact/runtime digest binding, component output rollback, and target runtime
evidence. It covers only the producer's multi-fixture staging transaction.

## Decision

For a multi-fixture batch, each producer records the per-fixture work and
separate runtime directories it creates. On any producer error before the final
report write, it removes only those task-owned directories in reverse creation
order. Existing work/runtime roots and caller-owned entries are preserved.
Successful batches retain their isolated artifacts for inspection and write
the report atomically as before. The ordinary single-fixture root semantics and
real filesystem crash durability are outside this slice.

## Evidence

The same late-failure fixture is used for both producers. The first valid
fixture creates staging, the second valid fixture fails deterministically, no
report is written, both generated directories are removed, and a pre-existing
caller sentinel remains:

```text
python3 scripts/ci/test-semantic-fixture-rust-report.py
python3 scripts/ci/test-semantic-fixture-native-report.py
```

This is offline/fake producer evidence. It does not prove current-source
native stage0 execution, component instantiation, Mac/Linux runtime parity,
packaged/rollback bytes parity, live provider/auth acquisition, or crash/power
loss filesystem semantics.

## Consequences

Rust/native producer failures now leave an all-or-nothing multi-fixture staging
boundary without deleting caller-owned state. EC-M3-04 / EC-M3-05 and
M3-04-N1 / M3-05-N9 remain `[~]` until current target artifacts and runtime
evidence are available.
