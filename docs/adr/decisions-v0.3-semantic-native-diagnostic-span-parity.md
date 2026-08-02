# ADR: Native semantic report diagnostic span parity

## Status

Accepted as a verified partial slice for EC-M3-04 / EC-M3-05. Full current-source
Rust/native producer parity, target runtime, and packaged evidence remain open.

## Context

The Rust semantic report producer accepts an explicit diagnostic code together
with either a compact byte range (`(start..end)`) or the structured span form
emitted by the Rust diagnostic renderer (`Span { start: ..., end: ... }`). The
native producer only recognized the compact form, so an otherwise equivalent
native diagnostic could not become a report and the Rust/native differential
lane would stop before comparing the observable span.

This is a diagnostic producer parity contract, distinct from source-commit
admission, runtime receipts, component byte identity, and static
source/ftable/import projection.

## Decision

Use the same structured span grammar in
`scripts/ci/semantic_fixture_native_report.py` as the Rust report producer:
accept compact ranges and multiline structured spans, including the optional
diagnostic gutter marker, then normalize both to the existing byte-offset based
`line`/`column` JSON shape. Missing code, missing span, reversed ranges, and
invalid UTF-8 boundaries remain fail-closed. No report schema fields or
diagnostic semantics are changed.

## Evidence

The native fake runner now emits the same multiline structured span fixture as
the Rust producer test (`LS3001`, offsets 214..216), and both producers assert
the same normalized diagnostic code and source span:

```text
python3 scripts/ci/test-semantic-fixture-rust-report.py
python3 scripts/ci/test-semantic-fixture-native-report.py
```

This is offline producer evidence only. It does not prove current Mac/Linux
stage0 execution, component instantiation, Wasm runtime behavior, or packaged /
rollback parity.

## Consequences

The differential lane can compare the existing closed diagnostic shape when
Rust and native render the same structured span. EC-M3-04 / EC-M3-05 and
M3-04-N1 / M3-05-N9 remain `[~]` until current target producer and runtime
evidence is available.
