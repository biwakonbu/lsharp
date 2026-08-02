# ADR: Exact source-to-artifact ABI expectation

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. The full
Rust/native producer and target-runtime requirement remains open.

## Context

The existing `v4-m1-07` static projection checked that a fixture declaring
`imports` and `ftable` produced non-empty imports and tables, then compared
Rust/native sidecars. That could accept the same wrong ABI from both
producers. The report schema and the existing runtime receipt are separate
contracts and must remain unchanged.

The current checkout has no current-source Linux replay manifest/expected lock,
and another session owns the Lima/QEMU/replayd resources. This slice therefore
uses the existing offline fake artifact boundary only.

## Decision

Add the sidecar contract
`scripts/ci/semantic-fixture-artifact-expectations.json` for
`valid/nested-record-pattern`. The static projection loads the regular
expectation file before emitting evidence and requires exact ordered equality
for `imports`, table/ftable shape, and `exports`. A shape drift such as a table
minimum changing from `3` to `4` fails before the new sidecar is written.

The expectation file is deliberately outside the semantic report schema. It
is a fixture-level ABI oracle for the existing projection, not a claim that
the Rust and native producers have already emitted this ABI on Mac and Linux.
The existing sidecar diff remains responsible for Rust/native exact comparison
and report/runtime digest binding.

## Evidence

The same fake `wasm-tools print` fixture now passes the exact expected import,
table, and export shape. A fake artifact with the table minimum changed from
`3` to `4` is rejected and leaves no projection output:

```text
python3 scripts/ci/test-semantic-fixture-artifact-projection.py
```

This is offline/fake evidence only. It does not prove current-source Rust or
native producer parity, component instantiation, Mac/Linux runtime behavior,
or packaged/rollback parity.

## Consequences

Both producers must first match one canonical fixture ABI before their
sidecars can become evidence. This closes the previous non-empty-only gap
without adding fields to the report schema or invoking a runtime. Additional
fixtures require explicit expectation entries and the same producer/runtime
evidence gates; EC-M3-04 / EC-M3-05 and M3-04-N1 / M3-05-N9 remain `[~]`.
