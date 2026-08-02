# Decision: Bind native fixture reports to the declared stage0 compiler

- Status: Accepted (verified partial)
- Date: 2026-08-02
- Scope: `scripts/ci/semantic_fixture_native_report.py`

## Context

The native semantic report producer validated the stage0 manifest kind,
target, source commit, and the shape of its three relative payload paths, but
it executed the separately supplied `--runner` without proving that the
runner was the compiler declared by the manifest. A report could therefore
describe one stage0 package while observing a different executable.

## Decision

Before any fixture or external helper is invoked, require the manifest's
declared `compiler` path to be a regular executable file and require its
resolved path to equal the explicit `--runner` path. A missing, symlinked,
non-executable, or mismatched compiler fails closed with no report and no
compile/runtime invocation. The `transport_driver` and `materializer` path
fields retain their existing safe-relative-shape validation; their execution
belongs to the stage0 runner contract.

## Evidence

The RED fake fixture supplied an executable runner while declaring
`bin/other-compiler` in the stage0 manifest. The old producer ran the unbound
runner and failed only later. The GREEN producer rejects the identity mismatch
before compilation. The native producer suite passes 20 tests and the
Rust/native report diff suite passes 9 tests.

## Boundary and follow-up

This is an offline native manifest-to-runner identity boundary. It does not
claim a current-source Mac/Linux stage0 runtime, full Rust/native producer
parity, packaged/rollback bytes parity, live provider/auth acquisition, or
actual Linux replay. No current-source manifest/expected replay lock was
available and another session owns Lima/QEMU/replayd, so heavy replay, stage
regeneration, and full build remain unrun. Reproduce the focused contract with
`python3 scripts/ci/test-semantic-fixture-native-report.py SemanticFixtureNativeReportTest.test_rejects_runner_not_bound_to_stage0_manifest`
and recheck the blocker with
`ps -axo pid=,command= | rg 'lsharp-linux-x86|replayd'`.
