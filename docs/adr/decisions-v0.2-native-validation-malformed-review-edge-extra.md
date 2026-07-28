# ADR: v0.2 native validation malformed review edge extra-arity rejection

- Status: Accepted (verified partial slice)
- Date: 2026-07-28
- Scope: `scripts/ci/native-selfhost-dev-source-file-smoke.sh`
- Related: `EC-M2-02`、`EC-M3-01`、`docs/adr/decisions-v0.2-native-validation-malformed-review-edge.md`

## Context

Review edges have exactly two wire endpoints after the relation name. The native smoke already
rejected a missing endpoint for both `evaluates` and `invalidates`, but an extra endpoint could drift
outside the parser-level fail-closed contract unless both relations were covered explicitly.

## Decision

- Add an `:evaluates "review:checkout/registered" "claim:checkout/rejects" "extra"` fixture.
- Add an `:invalidates "change:checkout/api-v2" "evidence:checkout/review" "extra"` fixture.
- The Rust syntax oracle rejects both fixtures with `LS0101`.
- Native `validate --source <fixture> --format json --emit-manifest <path>` invocations return exit `1`,
  stderr `source validation error:1`, empty stdout, and no manifest file.

## Evidence

- RED: `bash scripts/ci/test-native-linux-x86-native-stage0-source-file-smoke.sh` failed because
  the extra-arity fixture variables and contracts were absent from the inner smoke.
- GREEN: `cargo test -p lsharp-syntax --test intent_edges review_and_change_edge_metadata_reject_extra_wire_ids`
  passed, and the native source-file provenance test returned `Linux native stage0 source-file provenance tests: OK`
  under the fake Lima/provenance harness.
- `bash -n`, native selfhost runner tests, docs audit, and `git diff --check` passed.

## Boundary and follow-up

This is a parser/native source-file smoke contract only. It does not prove current-source packaged stage0
execution, Mac/Linux artifact/runtime parity, manifest bytes, or fallback exclusion. Keep `EC-M2-02`,
`EC-M2-03`, and M3 aggregate `[~]` until actual stage0 replay covers the same fixtures.
