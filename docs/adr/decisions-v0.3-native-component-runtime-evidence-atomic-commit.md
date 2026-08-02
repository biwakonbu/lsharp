# ADR: v0.3 native component runtime evidence atomic commit

## Context

The native component helper already validated the temporary component, optionally ran an
explicit `wasmtime`, and wrote a runtime-evidence sidecar before promoting the component
output. That ordering left a partial observable state possible: if the later output
promotion failed, the evidence could claim a runtime for an output that was not installed.
The reverse ordering would have the symmetric problem when evidence commit failed.

## Decision

Treat the component output and explicit runtime-evidence sidecar as one bounded local
transaction. The helper writes evidence to a sibling temporary file, moves an existing
regular-file or symlink output to a sibling backup, promotes the component, then promotes
the evidence. Any failure before both promotions complete removes the new output and
sidecar and restores the previous output. The test-only
`LSHARP_TEST_COMPONENT_FAILPOINT=output-promote|evidence-promote` controls deterministic
failure injection and is not a CLI/API option.

The normal route remains unchanged: runtime evidence still requires an explicit
`--wasmtime`, the component is validated before runtime, and no evidence is generated
implicitly. The transaction only covers local output/evidence promotion; it does not claim
crash consistency or power-loss durability across filesystems.

## Evidence

- RED: the two failpoints were initially accepted as success, so the existing output and
  evidence relationship was not protected.
- GREEN: `scripts/ci/test-native-selfhost-component.py` verifies both output-promotion and
  evidence-promotion failures preserve the existing component, leave no final sidecar, and
  clean component/evidence temporary paths. The pre-existing success, validator, runtime,
  mutation, child-failure, and no-fallback cases remain green.
- Verification uses fake native, `wasm-tools`, and `wasmtime` executables only. It does not
  prove current-source Mac/Linux runtime, real component instantiation, Rust/native producer
  parity, packaged/rollback parity, or provider/auth acquisition.

## Consequences

Runtime evidence cannot be published without its corresponding component promotion in this
helper's bounded local failure model. `EC-M3-04`, `EC-M3-05`, `M3-04-N1`, and `M3-05-N9`
remain `[~]` until real target runtime and packaged provenance evidence are available.
