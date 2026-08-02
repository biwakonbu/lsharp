# Decision: Admit invalid semantic fixtures only with the canonical compiler exit

- Status: Accepted (verified partial)
- Date: 2026-08-02
- Scope: Rust-oracle and native-stage0 semantic fixture report producers

## Context

Invalid fixtures already required a non-zero compiler result and a parseable
diagnostic. That allowed a producer to publish a report when a fake or future
compiler returned an unexpected non-zero exit, even though the fixture
manifest declared a different canonical compile outcome. Diagnostic code/span
parity and runtime exit/output admission are separate contracts; neither
made this invalid-fixture compiler boundary exact.

## Decision

For an invalid fixture, the Rust and native producers must compare the compiler
return code with `expected.exit_code` before parsing/publishing the report.
An unexpected non-zero exit fails closed with a compile-exit diagnostic, does
not create a report, and does not run Wasmtime. The existing diagnostic
parser, source fingerprint check, no-artifact rule, and invalid report schema
remain unchanged. The producer continues to reject a zero exit as before.

## Evidence

The same fake `invalid/type-undefined-value` fixture first demonstrated RED:
an exit of `2` with the expected `LS1001` diagnostic was accepted by both
producers. After the guard was added, the test is GREEN for both Rust and
native producers. The complete focused suites pass with Rust 18 tests and
native 19 tests.

## Boundary and follow-up

This verifies only the offline invalid-fixture compiler-exit admission. It is
not evidence for real current-source Mac/Linux runtime execution, packaged or
rollback parity, live provider/auth acquisition, or full Rust/native producer
parity. Those EC-M3-04/05 and M3-04-N1/M3-05-N9 boundaries remain `[~]` in
the active planning and TODO records. A current-source manifest and expected
replay lock were not available, and another session owns the Lima/QEMU/replayd
processes, so Linux replay, stage regeneration, and full build were not run.
RED/GREEN reproduction commands are
`python3 scripts/ci/test-semantic-fixture-rust-report.py SemanticFixtureRustReportTest.test_rejects_invalid_fixture_with_unexpected_compile_exit_before_report`
and the corresponding native test. Recheck the replay blocker before any
future heavy gate with
`ps -axo pid=,command= | rg 'lsharp-linux-x86|replayd'` and
`find . -path './target' -prune -o -type f \( -name manifest.json -o -name '*replay*lock*' -o -name 'expected-lock*' \) -print`.
