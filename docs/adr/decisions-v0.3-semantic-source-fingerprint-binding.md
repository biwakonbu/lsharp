# ADR: Semantic fixture source fingerprint binding

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Current-source
Rust/native producer parity and target-runtime evidence remain open.

## Context

The semantic report producers already admitted a requested `source_commit`, but
that identifies only the checkout `HEAD`. A working-tree source mutation could
be compiled by both producers and then pass differential comparison because
the reports had no binding to the fixture bytes actually observed.

This is distinct from source-commit admission and from the exact artifact ABI
expectation. It binds the compiler input, report, and current fixture content;
it does not claim that the artifact was produced by a current Mac/Linux native
stage0 runtime.

## Decision

Rust-oracle and native-stage0 report producers record a `source_sha256` for
every selected fixture before compiling its task-owned copy. Before returning
an observation, they recompute the source fixture digest and reject any source
mutation without writing a report. The Rust/native diff validates the digest
format, requires exact equality between producers, and recomputes the digest
from the current fixture before comparison. A stale or mismatched source
fingerprint therefore fails closed before evidence can be promoted.

## Evidence

The same fake producer fixtures cover positive source fingerprints, source
mutation during runtime, current-fixture digest mismatch, report differential,
evidence audit, and two-target aggregate compatibility:

```text
python3 scripts/ci/test-semantic-fixture-rust-report.py
python3 scripts/ci/test-semantic-fixture-native-report.py
python3 scripts/ci/test-semantic-fixture-diff.py
python3 scripts/ci/test-semantic-fixture-evidence-audit.py
python3 scripts/ci/test-semantic-fixture-evidence-aggregate.py
```

This is offline/fake and producer-boundary evidence. It does not prove native
stage0 execution, component instantiation, current Mac/Linux runtime parity,
packaged/rollback parity, or live provider/auth acquisition.

## Consequences

Reports now carry the fixture input fingerprint required for source-to-producer
differential comparison, while the existing commit, artifact, runtime, and
exact ABI contracts remain separate. EC-M3-04 / EC-M3-05 and M3-04-N1 /
M3-05-N9 remain `[~]` until current target artifacts and runtime evidence are
available.
