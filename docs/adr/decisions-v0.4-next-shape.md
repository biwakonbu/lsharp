# ADR: v0.4 native-first executable language shape

## Status

Proposed (2026-08-01). This ADR defines the next-version direction; it does
not close any current v0.3 or legacy TODO item.

## Context

L# already has verified slices for Rust/selfhost contracts, native MCP
projections, stage0 provenance, and several parser/type/runtime boundaries. The
remaining gaps are not one missing feature: they cross compiler semantics,
public commands, two supported targets, package/provider boundaries, and
release rollback. Treating a single Rust test or a stale stage0 as completion
would hide those gaps and recreate the branch/worktree conflict the project is
trying to avoid.

## Decision

- Define v0.4 as a native-first development loop whose semantic completion path
  is parser → type inference → lowering/module graph → codegen/ABI → runtime →
  public command → artifact/evidence identity.
- Keep Rust as bootstrap/oracle/rollback/host-integration infrastructure, while
  forbidding implicit Rust, host compiler, network, or provider fallback in a
  native success path.
- Limit product/release completion targets to `aarch64-apple-darwin` and
  `x86_64-unknown-linux-gnu`.
- Make every task an observable contract with a RED fixture, Rust differential,
  native stage0 evidence, target/runtime evidence, negative boundary, and ADR/
  TODO audit. Partial parity remains `[~]`; only full-scope tasks leave TODO.
- Treat provider acquisition/authentication as a caller-owned external
  snapshot boundary and treat package/install, LSP/MCP/REPL/doc, and rollback
  as first-class public surfaces rather than optional polish.
- Track the plan in `v0.4-lsharp-next-shape.md` and
  `v0.4-milestone-01.md`; do not activate its checkboxes until v0.3 ownership
  and dependency review permits the work.

## Consequences

- Future work is scheduled around semantic and evidence boundaries instead of
  isolated implementation layers.
- A verified slice can be merged independently without being mistaken for
  full Rust-free completion.
- Linux VM and stage regeneration remain serialized shared resources, so
  non-shared fixtures/docs/diagnostics are the default parallel work.
- Current v0.3 and legacy aggregate items stay visible until their own
  completion evidence is complete.

## Alternatives rejected

- Remove Rust immediately: this destroys bootstrap/oracle/rollback safety and
  does not prove native semantic parity.
- Declare v0.4 complete after native MCP or one target: this confuses a local
  projection with the full compiler/runtime/public/release contract.
- Add every future task directly to active TODO now: this mixes deferred work
  with current v0.3 truth and makes ownership/priority ambiguous.
