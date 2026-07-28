# ADR: v0.2 native validation whitespace-only source node text

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `selfhost/src/Tools/Validation/IntentSource.ls`, `crates/lsharp-types/src/validation_source/source_nodes.rs`, `crates/lsharp-types/tests/validation_source/nodes.rs`, `crates/lsharp-wasm/tests/e2e/selfhost_intent_source_adapter.rs`, `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-01`、`docs/adr/decisions-v0.2-intent-ast.md`

## Context

Intent graph node descriptions must contain meaningful text. Rust canonical `IntentNode::new`
rejects `text.trim().is_empty()` with `NodeTextError::EmptyText`, while selfhost `IntentSource`
previously checked only string length, allowing a whitespace-only `:claim` body to become a valid node.
When a malformed stable ID and whitespace-only text appeared together, Rust parsed the ID first while
selfhost/native rejected the required text first. That precedence made the source adapter boundaries
observable differently even though the standalone whitespace fixture used the same code.

## Decision

- Reject whitespace-only node descriptions before constructing a selfhost source node; keep stable-ID
  syntax validation on its existing invalid-ID path.
- Check `text.trim().is_empty()` before Rust stable-ID parsing in the source adapter, returning
  `NodeTextError::EmptyText` for the combined invalid-ID/blank-text fixture.
- Mirror the existing Rust/selfhost whitespace contract with a `source-node-nonblank?` helper that
  recognizes space, tab, LF, and CR.
- Preserve the source adapter malformed error code `1`, node kind `claim` (`7`), stable ID, exit `1`,
  empty stdout, and no-manifest fail-closed boundary.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because the
  whitespace-node-text fixture variables and contract were absent from the inner smoke.
- Differential RED: the Rust source test initially returned stable-ID failure for an invalid ID paired
  with whitespace-only text; the Rust-host selfhost E2E returned malformed code `1` first.
- Differential RED: the Rust-host selfhost E2E initially returned `["1"]` for a whitespace-only node
  body, while the expected malformed source result was `["0", "1", "7", "claim:checkout/whitespace-text"]`.
- Rust oracle: `cargo test -p lsharp-types --test validation_source source_adapter_rejects_whitespace_only_node_text`
  passed and asserts `NodeTextError::EmptyText`.
- Rust-host selfhost oracle:
  `cargo test -p lsharp-wasm --test e2e e2e::selfhost_intent_source_adapter::test_e2e_selfhost_source_adapter_rejects_whitespace_only_node_text -- --exact --nocapture`
  passed after the helper was added.
- Combined invalid-ID/blank-text Rust source and selfhost E2E tests pass with malformed code `1`; native
  source-file smoke covers the same fixture and no-manifest boundary.
- Native source-file provenance smoke, runner tests, docs audit, and `git diff --check` are required
  gates under the fake Lima/provenance harness.

## Boundary and follow-up

This closes whitespace and required-text precedence for source node ID/text validation only. It does not
prove current-source packaged stage0 execution, Mac/Linux artifact/runtime parity, manifest bytes, or
fallback exclusion. Keep `EC-M2-01` and M2/M3 aggregate `[~]`.
