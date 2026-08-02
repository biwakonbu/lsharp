# ADR: v0.3 native public unsupported compile preflight

## Context

The native selfhost development runner already rejected Rust-only compile/build targets
(`web-wasm`, `native`) and the Rust-only `--emit-ir` option. Before this slice, that
diagnostic was reached only after stage0 manifest validation and, when the generated stage
was not ready, transport, decoder, and materializer execution. A rejected public command
could therefore create a stage directory and invoke stage0 helpers even though no native
program should run.

## Decision

Run the unsupported compile/build target and option checks immediately after the basic
source/entry/decoder input checks and before stage0 manifest parsing, stage directory
creation, helper execution, environment handoff, or output routing. Preserve the existing
diagnostics and supported-target guidance. The preflight is an explicit native external
boundary: `web-wasm`, `native`, and `--emit-ir` remain Rust-host integration routes and are
never silently delegated to a native fallback.

## Evidence

- RED: with a fresh stage directory and an existing output sentinel, unsupported
  `--target web-wasm` bootstrapped stage0 state before returning its diagnostic.
- GREEN: `scripts/ci/test-native-selfhost-dev.sh` uses the same fake stage0 fixture to prove
  both unsupported target and `--emit-ir` reject without transport/materializer/helper
  invocation, without creating the stage directory, and without changing the existing
  output sentinel. The pre-existing supported compile/build routing and no-host-fallback
  checks remain green.
- Rust `EmbeddedCli` and its explicit component/Preview1 boundary are unchanged; this is
  focused native runner admission evidence, not Rust/native compiler implementation parity.
- No current-source Mac/Linux runtime, current artifact/expected replay lock, staged release
  package, full native producer parity, live provider/auth, or real Ed25519 verification was
  run or claimed.

## Consequences

Rejected public compile/build inputs no longer mutate native bootstrap state before the
caller receives the stable boundary diagnostic. Supported native routes retain their
existing stage0 and component-helper behavior. The public command and target matrix remain
partial; `EC-M3-03`, `EC-M3-04`, `EC-M3-05`, `M3-04-N1`, and `M3-05-N9` stay `[~]` until
current-source target/runtime and packaged evidence are available.
